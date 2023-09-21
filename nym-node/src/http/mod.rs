// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod middleware;
pub mod router;
pub mod state;

pub use router::{Config, NymNodeRouter, api, landing_page, policy};

// pub struct Config {
//     router_config: router::Config,
//     bind_address: SocketAddr,
// }
//
// async fn run(config: Config) -> Result<(), NymNodeError> {
//     NymNodeRouter::new(config.router_config)
//         .start_server(&config.bind_address)
//         .await
// }
