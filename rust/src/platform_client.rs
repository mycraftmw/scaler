use anyhow::{bail, Context, Result};
use tokio::time::Instant;
use tonic::transport::Channel;

use crate::{
    model::instance::Instance,
    pb::serverless_simulator::{
        platform_client::PlatformClient, CreateSlotRequest, DestroySlotRequest, InitRequest, Meta,
        ResourceConfig, Slot, Status,
    },
};

pub async fn init_slot(
    mut platform_client: PlatformClient<Channel>,
    request_id: String,
    instance_id: String,
    slot: Slot,
    meta_data: Meta,
) -> Result<Instance> {
    let request = InitRequest {
        request_id,
        slot_id: slot.id.clone(),
        instance_id: instance_id.clone(),
        meta_data: Some(meta_data.clone()),
    };

    let reply = platform_client.init(request).await?.into_inner();
    match reply.status() {
        Status::Ok => Ok(Instance {
            instance_id,
            slot,
            create_time_ms: reply.create_time,
            init_duration_ms: reply.init_duration_in_ms,
            last_idle_time: Instant::now(),
            meta: meta_data,
        }),
        _ => bail!("init slot reply error, reply:{reply:?}"),
    }
}

pub async fn create_slot(
    mut platform_client: PlatformClient<Channel>,
    request_id: String,
    resource_config: Option<ResourceConfig>,
) -> Result<Slot> {
    let request = CreateSlotRequest {
        request_id,
        resource_config,
    };
    let reply = platform_client.create_slot(request).await?.into_inner();
    match reply.status() {
        Status::Ok => reply.slot.context("slot not found in reply"),
        _ => bail!("create slot reply error, reply:{reply:?}"),
    }
}

pub async fn destroy_slot(
    mut platform_client: PlatformClient<Channel>,
    request_id: String,
    slot_id: String,
    reason: Option<String>,
) -> Result<()> {
    let request = DestroySlotRequest {
        request_id,
        id: slot_id,
        reason,
    };

    let reply = platform_client.destroy_slot(request).await?.into_inner();
    match reply.status() {
        Status::Ok => Ok(()),
        _ => bail!("destroy slot reply error, reply:{reply:?}"),
    }
}
