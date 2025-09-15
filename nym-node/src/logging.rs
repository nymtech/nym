// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::logging::{default_tracing_env_filter, default_tracing_fmt_layer};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

pub(crate) fn granual_filtered_env() -> anyhow::Result<EnvFilter> {
    fn directive_checked(directive: impl Into<String>) -> anyhow::Result<Directive> {
        directive.into().parse().map_err(From::from)
    }

    let mut filter = default_tracing_env_filter();

    // these crates are more granularly filtered
    let filter_crates = ["defguard_wireguard_rs"];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{crate_name}=warn"))?);
    }
    Ok(filter)
}

pub(crate) fn setup_tracing_logger() -> anyhow::Result<()> {
    let stderr_layer =
        default_tracing_fmt_layer(std::io::stderr).with_filter(granual_filtered_env()?);

    cfg_if::cfg_if! {if #[cfg(feature = "tokio-console")] {
        // instrument tokio console subscriber needs RUSTFLAGS="--cfg tokio_unstable" at build time
        let console_layer = console_subscriber::spawn();

        tracing_subscriber::registry()
            .with(console_layer)
            .with(stderr_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(stderr_layer)
            .init();
    }}

    Ok(())
}
