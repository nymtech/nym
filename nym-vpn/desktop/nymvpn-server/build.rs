fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false)
        .compile(&["proto/nymvpn-server.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
