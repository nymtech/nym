// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

mod api;
mod storage;

#[tokio::main]
async fn main() {
    setup_logging();

    let base_dir = default_base_dir();
    let storage = storage::NetworkStatisticsStorage::init(&base_dir)
        .await
        .expect("Could not create network statistics storage");

    tokio::spawn(
        rocket::build()
            .mount(
                "/v1",
                rocket::routes![storage::routes::post_service_statistics],
            )
            .manage(storage.clone())
            .ignite()
            .await
            .expect("Could not ignite stats api service")
            .launch(),
    );
}

fn setup_logging() {
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
        .init();
}

/// Returns the default base directory for the storefile.
///
/// This is split out so we can easily inject our own base_dir for unit tests.
fn default_base_dir() -> PathBuf {
    dirs::home_dir()
        .expect("no home directory known for this OS")
        .join(".nym")
        .join("service-providers")
        .join("network-statistics")
}
