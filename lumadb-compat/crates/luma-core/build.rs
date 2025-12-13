fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .compile(
            &["../../../go-cluster/proto/luma.proto"],
            &["../../../go-cluster/proto"],
        )?;
    Ok(())
}
