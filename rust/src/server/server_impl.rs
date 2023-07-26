use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};
use tonic::{async_trait, transport::Channel, Request, Response, Status};
use tracing::instrument;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::pb::serverless_simulator::{
    platform_client::PlatformClient, scaler_server::Scaler, AssignReply, AssignRequest, Assignment,
    CreateSlotRequest, DestroySlotRequest, IdleReply, IdleRequest, InitRequest, Meta,
    ResourceConfig, Slot,
};

#[derive(Debug)]
pub struct ScalerServerImpl {
    free_slots: Arc<Mutex<mpsc::UnboundedReceiver<Slot>>>,
    slot_feed: mpsc::UnboundedSender<Slot>,
    instances: Arc<Mutex<HashMap<String, Slot>>>,
    platform_client: Arc<Mutex<PlatformClient<Channel>>>,
}

#[async_trait]
impl Scaler for ScalerServerImpl {
    #[instrument]
    async fn assign(
        &self,
        request: Request<AssignRequest>,
    ) -> Result<Response<AssignReply>, Status> {
        let AssignRequest {
            request_id,
            timestamp,
            meta_data,
        } = request.into_inner();

        // try queue
        if let Ok(slot) = self.free_slots.lock().await.try_recv() {
            let instance_id = Uuid::new_v4().to_string();
            self.instances
                .lock()
                .await
                .insert(instance_id.clone(), slot.clone());
            let mut assignment = Assignment {
                request_id: request_id.clone(),
                meta_key: String::new(),
                instance_id,
            };
            if let Some(meta) = meta_data {
                assignment.meta_key = meta.key.clone();
            }
            return Ok(Response::new(AssignReply {
                status: 0,
                assigment: Some(assignment),
                error_message: String::new(),
            }));
        } else {
            // create
            let resource_config = meta_data.clone().and_then(|meta| {
                Some(ResourceConfig {
                    memory_in_megabytes: meta.memory_in_mb,
                })
            });
            info!("try create a slot");
            let _ = self.create_slot(request_id.clone(), resource_config).await;
        }
        // waiting for free slot
        if let Some(slot) = self.free_slots.lock().await.recv().await {
            let instance_id = Uuid::new_v4().to_string();
            self.instances
                .lock()
                .await
                .insert(instance_id.clone(), slot.clone());
            let mut assignment = Assignment {
                request_id,
                meta_key: String::new(),
                instance_id,
            };
            if let Some(meta) = meta_data {
                assignment.meta_key = meta.key.clone()
            }
            return Ok(Response::new(AssignReply {
                status: 0,
                assigment: Some(assignment),
                error_message: String::new(),
            }));
        }
        Ok(Response::new(AssignReply {
            status: -1,
            assigment: None,
            error_message: "busy".to_string(),
        }))
    }

    #[instrument]
    async fn idle(&self, request: Request<IdleRequest>) -> Result<Response<IdleReply>, Status> {
        let IdleRequest {
            assigment: assignment,
            result,
        } = request.into_inner();

        if let Some(assignment) = assignment {
            // if the assignment is valid
            if let Some(slot) = self.instances.lock().await.remove(&assignment.instance_id) {
                if let Some(result) = result {
                    if result.need_destroy {
                        let request_id = Uuid::new_v4().to_string();
                        let _ = self.destroy_slot(request_id, slot.id, result.reason).await;
                        return Ok(Response::new(IdleReply::default()));
                    }
                }
                // recycling
                let _ = self.slot_feed.send(slot);
            }
            return Ok(Response::new(IdleReply::default()));
        }

        Ok(Response::new(IdleReply::default()))
    }
}

impl ScalerServerImpl {
    pub async fn new() -> Result<Self> {
        let (tx, rx) = mpsc::unbounded_channel();
        let platform_client = loop {
            info!("connecting to platform...");
            match PlatformClient::connect("http://127.0.0.1:50051").await {
                Ok(platform_client) => break platform_client,
                Err(err) => {
                    warn!("connecting failed: {err:?}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        };

        Ok(ScalerServerImpl {
            free_slots: Arc::new(Mutex::new(rx)),
            slot_feed: tx.clone(),
            instances: Arc::new(Mutex::new(HashMap::new())),
            platform_client: Arc::new(Mutex::new(platform_client)),
        })
    }

    pub async fn create_slot(
        &self,
        request_id: String,
        resource_config: Option<ResourceConfig>,
    ) -> Result<()> {
        let request = CreateSlotRequest {
            request_id: request_id.clone(),
            resource_config,
        };
        let reply = self
            .platform_client
            .lock()
            .await
            .create_slot(request)
            .await?
            .into_inner();
        match reply.slot {
            Some(slot) => {
                info!("slot created");
                let instance_id = Uuid::new_v4().to_string();
                if let Err(e) = self
                    .init_slot(instance_id, None, request_id.clone(), slot.id.clone())
                    .await
                {
                    error!("init failed: {e:?}");
                    // if init failed, destroy it
                    self.destroy_slot(request_id, slot.id, "failed to init".to_string())
                        .await?
                } else {
                    let _ = self.slot_feed.send(slot);
                }
            }
            None => {
                warn!("slot cannot be created, reply:{reply:?}");
            }
        }
        Ok(())
    }

    pub async fn destroy_slot(
        &self,
        request_id: String,
        slot_id: String,
        reason: String,
    ) -> Result<()> {
        let request = DestroySlotRequest {
            request_id,
            id: slot_id,
            reason,
        };
        let reply = self
            .platform_client
            .lock()
            .await
            .destroy_slot(request)
            .await?
            .into_inner();
        info!("destroy slot: {reply:?}");
        Ok(())
    }

    async fn init_slot(
        &self,
        instance_id: String,
        meta_data: Option<Meta>,
        request_id: String,
        slot_id: String,
    ) -> Result<()> {
        let request = InitRequest {
            instance_id,
            meta_data,
            request_id,
            slot_id,
        };
        let reply = self
            .platform_client
            .lock()
            .await
            .init(request)
            .await?
            .into_inner();
        info!("init success: {reply:?}");
        Ok(())
    }
}
