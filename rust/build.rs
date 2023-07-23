fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("PROTO").is_ok() {
        tonic_build::configure()
            .out_dir("src/pb")
            .build_server(true)
            .build_client(true)
            .compile(&["proto/serverless-sim.proto"], &["proto"])?;
    }

    Ok(())
}
