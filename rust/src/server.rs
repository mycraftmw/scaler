use crate::{
    pb::serverless_simulator::scaler_server::ScalerServer, server::server_impl::ScalerServerImpl,
};
use anyhow::Result;

mod server_impl;

pub async fn build_server() -> Result<ScalerServer<ScalerServerImpl>> {
    Ok(ScalerServer::new(ScalerServerImpl::new().await?))
}
