fn main() -> Result<(), Box<dyn std::error::Error>> {
    const PROTO_PATH: &str = "proto/nymvpn-controller.proto";
    tonic_build::configure().protoc_arg("--experimental_allow_proto3_optional")
        .compile(&[PROTO_PATH], &["proto"])?;
    println!("cargo:rerun-if-changed=proto");
    Ok(())
}
