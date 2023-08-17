pub mod server_impl;

use anyhow::Result;
use tonic::Status as TonicStatus;
use tonic::{async_trait, transport::Channel, Request, Response};
use tracing::{info, warn};

use crate::pb::serverless_simulator::Status;
use crate::pb::serverless_simulator::{
    platform_client::PlatformClient, scaler_server::Scaler, AssignReply, AssignRequest, IdleReply,
    IdleRequest,
};

#[derive(Debug)]
pub struct ScalerServerImpl {
    platform_client: PlatformClient<Channel>,
}

#[async_trait]
impl Scaler for ScalerServerImpl {
    async fn assign(
        &self,
        request: Request<AssignRequest>,
    ) -> Result<Response<AssignReply>, TonicStatus> {
        Ok(
            server_impl::assign_impl(self.platform_client.clone(), request)
                .await
                .unwrap_or_else(|err| {
                    let reply = AssignReply {
                        status: Status::InternalError as i32,
                        assigment: None,
                        error_message: Some(err.to_string()),
                    };
                    Response::new(reply)
                }),
        )
    }

    async fn idle(
        &self,
        request: Request<IdleRequest>,
    ) -> Result<Response<IdleReply>, TonicStatus> {
        Ok(
            server_impl::idle_impl(self.platform_client.clone(), request)
                .await
                .unwrap_or_else(|err| {
                    let reply = IdleReply {
                        status: Status::InternalError as i32,
                        error_message: Some(err.to_string()),
                    };
                    Response::new(reply)
                }),
        )
    }
}

impl ScalerServerImpl {
    pub async fn new() -> Result<Self> {
        let platform_client = loop {
            info!("connecting to platform...");
            match PlatformClient::connect("http://127.0.0.1:50051").await {
                Ok(platform_client) => break platform_client,
                Err(err) => {
                    warn!("connecting failed: {err:?}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        };
        info!("platform connected!");
        Ok(ScalerServerImpl { platform_client })
    }
}
