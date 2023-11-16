fn main() -> Result<(), Box<dyn std::error::Error>> {
    const PROTO_PATH: &str = "proto/nymvpn-controller.proto";
    tonic_build::configure().compile(&[PROTO_PATH], &["proto"])?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
