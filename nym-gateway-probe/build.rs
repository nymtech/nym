// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![allow(clippy::panic)]

use anyhow::{Context, bail};
use std::{path::PathBuf, process::Command};
use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder, RustcBuilder};

fn main() -> anyhow::Result<()> {
    build_go()?;
    generate_exit_policy_ports()?;

    Emitter::default()
        .add_instructions(&BuildBuilder::all_build()?)?
        .add_instructions(&CargoBuilder::all_cargo()?)?
        .add_instructions(&GitclBuilder::all_git()?)?
        .add_instructions(&RustcBuilder::all_rustc()?)?
        .emit()
}

/// Parse PORT_MAPPINGS from network-tunnel-manager.sh and generate a sorted
/// Rust const with every unique port. Ranges are represented by their start
/// and end values so a single TCP check can confirm the iptables rule exists.
/// TODO: consider runtime fetch from NS API exit policy endpoint instead of parsing the script
fn generate_exit_policy_ports() -> anyhow::Result<()> {
    use std::collections::BTreeMap;

    let script_path = PathBuf::from("../scripts/nym-node-setup/network-tunnel-manager.sh");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").context("OUT_DIR not set")?);

    println!("cargo::rerun-if-changed={}", script_path.display());

    let content = std::fs::read_to_string(&script_path).context(
        "failed to read network-tunnel-manager.sh — is it present at ../scripts/nym-node-setup/ ?",
    )?;

    // port → service name (BTreeMap keeps them sorted)
    let mut port_map: BTreeMap<u16, String> = BTreeMap::new();
    let mut in_mappings = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("declare -A PORT_MAPPINGS=(") {
            in_mappings = true;
            continue;
        }
        if in_mappings && trimmed == ")" {
            break;
        }
        if !in_mappings {
            continue;
        }

        // strip comment prefix so we still pick up ports that are opened
        // via a separate mechanism (e.g. SMTP/465 with rate limiting)
        let stripped = trimmed.trim_start_matches('#').trim();

        // match ["ServiceName"]="port-or-range"
        let Some(name_start) = stripped.find("[\"") else {
            continue;
        };
        let Some(name_end) = stripped.find("\"]=") else {
            continue;
        };
        let service = &stripped[name_start + 2..name_end];
        let value = stripped[name_end + 3..].trim_matches('"');

        if value.contains('-') {
            let parts: Vec<&str> = value.split('-').collect();
            if parts.len() == 2 {
                if let (Ok(lo), Ok(hi)) = (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                    port_map
                        .entry(lo)
                        .or_insert_with(|| format!("{service} (range start)"));
                    port_map
                        .entry(hi)
                        .or_insert_with(|| format!("{service} (range end)"));
                }
            }
        } else if let Ok(port) = value.parse::<u16>() {
            port_map.entry(port).or_insert_with(|| service.to_string());
        }
    }

    if port_map.is_empty() {
        bail!("No ports found in PORT_MAPPINGS — is network-tunnel-manager.sh correct?");
    }

    // write generated Rust source
    let mut out = String::new();
    out.push_str(
        "// Auto-generated from scripts/nym-node-setup/network-tunnel-manager.sh PORT_MAPPINGS.\n",
    );
    out.push_str("// Do not edit — changes are overwritten on rebuild.\n");
    out.push_str("// To add or remove ports, update PORT_MAPPINGS in the shell script.\n\n");
    out.push_str(&format!(
        "/// {} unique ports parsed from the canonical exit policy at build time.\n",
        port_map.len()
    ));
    out.push_str("pub const EXIT_POLICY_PORTS: &[u16] = &[\n");
    for (port, service) in &port_map {
        let entry = format!("{port},");
        out.push_str(&format!("    {entry:<7}// {service}\n"));
    }
    out.push_str("];\n");

    std::fs::write(out_dir.join("exit_policy_ports.rs"), out)?;
    Ok(())
}

fn build_go() -> anyhow::Result<()> {
    const LIB_NAME: &str = "netstack_ping";

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").context("target os is not set")?;
    // Only build on macos and linux
    if !matches!(target_os.as_str(), "macos" | "linux") {
        return Ok(());
    }

    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").context("target arch is not set")?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").context("OUT_DIR is not set")?);
    let go_target = match target_os.as_str() {
        "macos" => "darwin".to_owned(),
        "linux" => target_os.to_owned(),
        _ => panic!("unsupported target: {target_os}"),
    };
    let go_arch = match target_arch.as_str() {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        _ => panic!("unsupported architecture: {target_arch}"),
    };
    let src_dir = PathBuf::from("netstack_ping").canonicalize()?;
    let binary_out_path = out_dir.join(format!("lib{LIB_NAME}.a"));

    println!("cargo::rerun-if-changed={}", src_dir.display());

    let mut command = Command::new("go");

    if target_os == "macos" {
        let deployment_target =
            std::env::var_os("MACOSX_DEPLOYMENT_TARGET").unwrap_or("10.13".into());
        command.env("MACOSX_DEPLOYMENT_TARGET", deployment_target);
    }

    let mut child = command
        .env("CGO_ENABLED", "1")
        .env("GOOS", go_target)
        .env("GOARCH", go_arch)
        .current_dir(src_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .arg("build")
        .arg("-ldflags=-buildid=")
        .arg("-trimpath")
        .arg("-buildvcs=false")
        .arg("-v")
        .arg("-o")
        .arg(binary_out_path)
        .arg("-buildmode")
        .arg("c-archive")
        // Include all Go source files in the package (except tests)
        .arg("lib.go")
        .arg("udp_forwarder.go")
        .spawn()?;
    let status = child.wait()?;
    if !status.success() {
        bail!("Failed to build {LIB_NAME}");
    }

    println!("cargo::rustc-link-search={}", out_dir.display());
    println!("cargo:rustc-link-lib=static={LIB_NAME}");

    Ok(())
}
