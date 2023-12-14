// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct NyxdClient {
    inner: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
}

impl NyxdClient {
    pub(crate) fn new(config: &Config) -> Self {
        let details = config.get_network_details();
        let nyxd_url = config.get_nyxd_url();

        let client_config = nyxd::Config::try_from_nym_network_details(&details)
            .expect("failed to construct valid validator client config with the provided network");

        let mnemonic = config.base.mnemonic.clone();

        let inner = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            nyxd_url.as_str(),
            mnemonic,
        )
        .expect("Failed to connect to nyxd!");

        NyxdClient {
            inner: Arc::new(RwLock::new(inner)),
        }
    }
}
