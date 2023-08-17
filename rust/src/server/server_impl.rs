use anyhow::Result;
use tokio::time::Instant;
use tonic::Status as TonicStatus;
use tonic::{transport::Channel, Request, Response};
use tracing::{error, info};
use uuid::Uuid;

use crate::model::instance::{
    add_idle_instance, get_idle_instance, remove_from_instance_map, try_add_idle_instance,
    try_get_idle_instance, update_instance_map,
};
use crate::model::meta_data::insert_meta_data;
use crate::pb::serverless_simulator::Status as ServiceStatus;
use crate::pb::serverless_simulator::{
    platform_client::PlatformClient, AssignReply, AssignRequest, Assignment, IdleReply,
    IdleRequest, Meta, ResourceConfig,
};
use crate::platform_client;

pub async fn assign_impl(
    platform_client: PlatformClient<Channel>,
    request: Request<AssignRequest>,
) -> Result<Response<AssignReply>> {
    let request_start = Instant::now();
    let AssignRequest {
        request_id,
        meta_data,
        ..
    } = request.into_inner();
    info!("assign request receivied {request_id}");
    // update the meta data map
    let meta_data = meta_data.ok_or_else(|| TonicStatus::invalid_argument("no meta data found"))?;
    insert_meta_data(meta_data.clone()).await;
    // try from avaliable instance
    if let Some(instance) = try_get_idle_instance(&meta_data.key).await {
        update_instance_map(instance.clone()).await;
        let assignment = Assignment {
            request_id: request_id.clone(),
            meta_key: meta_data.key.clone(),
            instance_id: instance.instance_id.clone(),
        };
        let reply = AssignReply {
            status: ServiceStatus::Ok as i32,
            assigment: Some(assignment),
            error_message: None,
        };
        info!(
            "assign {instance:?} for request {request_id}, cost {:?}",
            request_start.elapsed()
        );
        return Ok(Response::new(reply));
    }

    // apply one
    let request_id_clone = request_id.clone();
    let meta_data_clone = meta_data.clone();
    let fut = async move {
        // todo: retry
        if let Err(err) =
            schedule_instance(platform_client, request_id_clone, meta_data_clone).await
        {
            error!("failed to schedule an new instance, err:{err}");
        }
    };
    tokio::spawn(fut);

    let instance = get_idle_instance(&meta_data.key).await.map_err(|err| {
        error!("cannot get available instance, err:{err}");
        TonicStatus::internal(err.to_string())
    })?;

    update_instance_map(instance.clone()).await;
    let assignment = Assignment {
        request_id: request_id.clone(),
        meta_key: meta_data.key.clone(),
        instance_id: instance.instance_id.clone(),
    };
    let reply = AssignReply {
        status: ServiceStatus::Ok as i32,
        assigment: Some(assignment),
        error_message: None,
    };
    info!(
        "assign {instance:?} for request {request_id}, cost {:?}",
        request_start.elapsed()
    );
    Ok(Response::new(reply))
}

async fn schedule_instance(
    platform_client: PlatformClient<Channel>,
    request_id: String,
    meta_data: Meta,
) -> Result<()> {
    let resource_config = ResourceConfig {
        memory_in_megabytes: meta_data.memory_in_mb,
    };
    let slot = platform_client::create_slot(
        platform_client.clone(),
        request_id.clone(),
        Some(resource_config),
    )
    .await?;
    let instance_id = Uuid::new_v4().to_string();

    let instance = platform_client::init_slot(
        platform_client.clone(),
        request_id.clone(),
        instance_id,
        slot,
        meta_data.clone(),
    )
    .await?;

    add_idle_instance(instance.clone()).await?;
    Ok(())
}

pub async fn idle_impl(
    platform_client: PlatformClient<Channel>,
    request: Request<IdleRequest>,
) -> Result<Response<IdleReply>> {
    let request_start = Instant::now();
    let IdleRequest { assigment, result } = request.into_inner();
    let assignment =
        assigment.ok_or_else(|| TonicStatus::invalid_argument("no assignment provided"))?;
    let Assignment {
        request_id,
        instance_id,
        ..
    } = assignment;
    let mut instance = remove_from_instance_map(instance_id.clone())
        .await
        .map_err(|err| TonicStatus::invalid_argument(err.to_string()))?;
    instance.last_idle_time = Instant::now();
    let need_destroy = result.map(|r| r.need_destroy()).unwrap_or_default();
    if need_destroy {
        let _ = platform_client::destroy_slot(
            platform_client.clone(),
            request_id.clone(),
            instance.slot.id.clone(),
            None,
        )
        .await;
        info!(
            "idle instance: {instance:?} destroyed, needed, cost: {:?}",
            request_start.elapsed()
        );
    } else {
        match try_add_idle_instance(instance.clone()).await {
            Ok(_) => info!(
                "idle instance: {instance:?} reused, cose: {:?}",
                request_start.elapsed()
            ),
            Err(_) => {
                // if buffer is full then destory this slot
                let _ = platform_client::destroy_slot(
                    platform_client,
                    request_id,
                    instance.slot.id.clone(),
                    None,
                )
                .await;
                info!(
                    "idle instance: {instance:?} destroyed, buffer is full, cost: {:?}",
                    request_start.elapsed()
                )
            }
        }
    }
    let reply = IdleReply {
        status: ServiceStatus::Ok as i32,
        error_message: None,
    };
    Ok(Response::new(reply))
}
