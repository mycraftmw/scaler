use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_max_level(Level::DEBUG)
        .init();
    let addr = "127.0.0.1:9001".parse()?;
    let svc = scaler::server::build_server().await?;
    info!("scaler service starting");
    tonic::transport::Server::builder()
        .max_concurrent_streams(1000)
        .add_service(svc)
        .serve(addr)
        .await?;
    Ok(())
}
