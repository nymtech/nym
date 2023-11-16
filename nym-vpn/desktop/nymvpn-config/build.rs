use std::error::Error;

use serde::Deserialize;

#[derive(Deserialize)]
struct Package {
    version: String,
}

#[derive(Deserialize)]
struct CargoToml {
    package: Package,
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = "../nymvpn-packages/Cargo.toml";
    println!("cargo:rerun-if-changed={path}");
    let cargo_toml: CargoToml = toml::from_str(&std::fs::read_to_string(path)?)?;
    println!(
        "cargo:rustc-env=NYMVPN_VERSION={}",
        cargo_toml.package.version
    );

    Ok(())
}
