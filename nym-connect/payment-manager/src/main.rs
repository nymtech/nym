// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::logging::setup_logging;
use nym_network_defaults::setup_env;
use nym_task::TaskManager;
use std::error::Error;

mod client;
mod error;
mod http;
mod state;
mod storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_logging();
    setup_env(None);

    // let's build our rocket!
    let rocket = http::setup_rocket().await?;

    // setup shutdowns
    let shutdown = TaskManager::new(10);
    let rocket_shutdown_handle = rocket.shutdown();

    // launch rocket
    tokio::spawn(rocket.launch());

    // wait for termination
    shutdown.catch_interrupt().await.ok();
    rocket_shutdown_handle.notify();

    Ok(())
}
