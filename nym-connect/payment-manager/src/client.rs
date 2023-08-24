// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) struct Client(pub(crate) Arc<RwLock<DirectSigningHttpRpcNyxdClient>>);

impl Clone for Client {
    fn clone(&self) -> Self {
        Client(Arc::clone(&self.0))
    }
}

impl Client {
    pub(crate) fn new(details: &NymNetworkDetails) -> Result<Self, Error> {
        let client_config = nyxd::Config::try_from_nym_network_details(details)?;
        let endpoint = details
            .endpoints
            .first()
            .ok_or(Error::EmptyValidatorList)?
            .nyxd_url
            .as_str();

        let inner = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            endpoint,
            "".parse().unwrap(),
        )?;

        Ok(Client(Arc::new(RwLock::new(inner))))
    }
}
