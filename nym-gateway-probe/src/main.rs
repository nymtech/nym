// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[cfg(unix)]
mod run;

use nym_http_api_client_macro::client_defaults;
use std::time::Duration;

client_defaults!(
    priority = 10;
    timeout = Duration::from_secs(300),
    local_address = std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
);

#[cfg(unix)]
#[tokio::main]
#[allow(clippy::exit)] // Intentional exit on error for CLI tool
async fn main() -> anyhow::Result<()> {
    match run::run().await {
        Ok(ref result) => {
            let json = serde_json::to_string_pretty(result)?;
            println!("{json}");
        }
        Err(err) => {
            eprintln!("An error occurred: {err}");
            std::process::exit(1)
        }
    }
    Ok(())
}

#[cfg(not(unix))]
#[tokio::main]
#[allow(clippy::exit)] // Intentional exit for unsupported platform
async fn main() -> anyhow::Result<()> {
    eprintln!("This tool is only supported on Unix systems");
    std::process::exit(1)
}
