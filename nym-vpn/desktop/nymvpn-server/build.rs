fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .protoc_arg(("--experimental_allow_proto3_optional"))
        .compile(&["proto/nymvpn-server.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
