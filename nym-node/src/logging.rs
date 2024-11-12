// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use tracing::level_filters::LevelFilter;
use tracing_subscriber::{filter::Directive, EnvFilter};

pub(crate) fn setup_tracing_logger() -> anyhow::Result<()> {
    fn directive_checked(directive: impl Into<String>) -> anyhow::Result<Directive> {
        directive.into().parse().map_err(From::from)
    }

    let log_builder = tracing_subscriber::fmt()
        // Use a more compact, abbreviated log format
        .compact()
        // Display source code file paths
        .with_file(true)
        // Display source code line numbers
        .with_line_number(true)
        // Don't display the event's target (module path)
        .with_target(false);

    let mut filter = EnvFilter::builder()
        // if RUST_LOG isn't set, set default level
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    // these crates are more granularly filtered
    let filter_crates = ["defguard_wireguard_rs"];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{}=warn", crate_name))?);
    }

    log_builder.with_env_filter(filter).init();

    Ok(())
}
