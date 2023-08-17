fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("PROTO").is_ok() {
        tonic_build::configure()
            .protoc_arg("--experimental_allow_proto3_optional")
            .out_dir("src/pb")
            .build_server(true)
            .build_client(true)
            .build_transport(true)
            .compile(&["proto/serverless-sim.proto"], &["proto"])?;
    }
    Ok(())
}
