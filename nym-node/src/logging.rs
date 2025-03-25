// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::logging::{default_tracing_env_filter, default_tracing_fmt_layer};
use tracing_subscriber::filter::Directive;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub(crate) fn granual_filtered_env() -> anyhow::Result<tracing_subscriber::filter::EnvFilter> {
    fn directive_checked(directive: impl Into<String>) -> anyhow::Result<Directive> {
        directive.into().parse().map_err(From::from)
    }

    let mut filter = default_tracing_env_filter();

    // these crates are more granularly filtered
    let filter_crates = ["defguard_wireguard_rs"];
    for crate_name in filter_crates {
        filter = filter.add_directive(directive_checked(format!("{}=warn", crate_name))?);
    }
    Ok(filter)
}

pub(crate) fn build_tracing_logger() -> anyhow::Result<impl SubscriberExt> {
    Ok(tracing_subscriber::registry()
        .with(default_tracing_fmt_layer(std::io::stderr))
        .with(granual_filtered_env()?))
}

pub(crate) fn setup_tracing_logger() -> anyhow::Result<()> {
    build_tracing_logger()?.init();

    Ok(())
}
