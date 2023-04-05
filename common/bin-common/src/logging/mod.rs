// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use tracing_subscriber::{
    fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

// I'd argue we should start transitioning from `log` to `tracing`
pub fn setup_logging() {
    let mut log_builder = pretty_env_logger::formatted_timed_builder();
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        log_builder.parse_filters(&s);
    } else {
        // default to 'Info'
        log_builder.filter(None, log::LevelFilter::Info);
    }

    log_builder
        .filter_module("hyper", log::LevelFilter::Warn)
        .filter_module("tokio_reactor", log::LevelFilter::Warn)
        .filter_module("reqwest", log::LevelFilter::Warn)
        .filter_module("mio", log::LevelFilter::Warn)
        .filter_module("want", log::LevelFilter::Warn)
        .filter_module("tungstenite", log::LevelFilter::Warn)
        .filter_module("tokio_tungstenite", log::LevelFilter::Warn)
        .filter_module("handlebars", log::LevelFilter::Warn)
        .filter_module("sled", log::LevelFilter::Warn)
        .init();
}

pub fn setup_tracing(file_name: &str) {
    let file_appender = tracing_appender::rolling::hourly(file_name, "log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let appender_layer = Layer::new().with_writer(non_blocking);

    let registry = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(appender_layer);

    registry.init();
}

pub fn banner(crate_name: &str, crate_version: &str) -> String {
    format!(
        r#"

      _ __  _   _ _ __ ___
     | '_ \| | | | '_ \ _ \
     | | | | |_| | | | | | |
     |_| |_|\__, |_| |_| |_|
            |___/

             ({crate_name} - version {crate_version})

    "#
    )
}

pub fn maybe_print_banner(crate_name: &str, crate_version: &str) {
    if atty::is(atty::Stream::Stdout) {
        println!("{}", banner(crate_name, crate_version))
    }
}
