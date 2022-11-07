// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use api::NetworkStatisticsAPI;
use logging::setup_logging;
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

    let api = NetworkStatisticsAPI::init(storage)
        .await
        .expect("Could not ignite stats api service");
    api.run().await;
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
