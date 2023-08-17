use std::time::Duration;

use scaler::{pb::serverless_simulator::scaler_server::ScalerServer, server::ScalerServerImpl};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let addr = "0.0.0.0:9001".parse()?;
    info!("scaler service starting");
    tonic::transport::Server::builder()
        .max_concurrent_streams(1000)
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .add_service(ScalerServer::new(ScalerServerImpl::new().await?))
        .serve(addr)
        .await
        .map_err(|e| {
            info!("scaler service shutting due to {e}");
            e
        })?;

    Ok(())
}
