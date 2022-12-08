// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Collection of initialization steps used by client implementations

use std::fmt::Display;

use nymsphinx::addressing::{clients::Recipient, nodes::NodeIdentity};
use serde::Serialize;
use tap::TapFallible;

use config::NymConfig;
use crypto::asymmetric::{encryption, identity};

use crate::{
    config::{persistence::key_pathfinder::ClientKeyPathfinder, Config, GatewayEndpointConfig},
    error::ClientCoreError,
    init::helpers::{query_gateway_details, register_with_gateway_and_store_keys},
};

mod helpers;

#[derive(Debug, Serialize)]
pub struct InitResults {
    version: String,
    id: String,
    identity_key: String,
    encryption_key: String,
    gateway_id: String,
    gateway_listener: String,
}

impl InitResults {
    pub fn new<T>(config: &Config<T>, address: &Recipient) -> Self
    where
        T: NymConfig,
    {
        Self {
            version: config.get_version().to_string(),
            id: config.get_id(),
            identity_key: address.identity().to_base58_string(),
            encryption_key: address.encryption_key().to_base58_string(),
            gateway_id: config.get_gateway_id(),
            gateway_listener: config.get_gateway_listener(),
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Version: {}", self.version)?;
        writeln!(f, "ID: {}", self.id)?;
        writeln!(f, "Identity key: {}", self.identity_key)?;
        writeln!(f, "Encryption: {}", self.encryption_key)?;
        writeln!(f, "Gateway ID: {}", self.gateway_id)?;
        write!(f, "Gateway: {}", self.gateway_listener)
    }
}

pub async fn setup_gateway<T: NymConfig>(
    register: bool,
    user_chosen_gateway_id: Option<&str>,
    config: &Config<T>,
) -> Result<GatewayEndpointConfig, ClientCoreError> {
    if register {
        // Get the gateway details by querying the validator-api. Either pick one at random or use
        // the chosen one if it's among the available ones.
        println!("Configuring gateway");
        let gateway =
            query_gateway_details(config.get_validator_api_endpoints(), user_chosen_gateway_id)
                .await?;
        log::debug!("Querying gateway gives: {}", gateway);

        // Registering with gateway by setting up and writing shared keys to disk
        log::trace!("Registering gateway");
        register_with_gateway_and_store_keys(gateway.clone(), config).await?;
        println!("Saved all generated keys");

        Ok(gateway.into())
    } else if user_chosen_gateway_id.is_some() {
        // Just set the config, don't register or create any keys
        // This assumes that the user knows what they are doing, and that the existing keys are
        // valid for the gateway being used
        println!("Using gateway provided by user, keeping existing keys");
        let gateway =
            query_gateway_details(config.get_validator_api_endpoints(), user_chosen_gateway_id)
                .await?;
        log::debug!("Querying gateway gives: {}", gateway);
        Ok(gateway.into())
    } else {
        Err(ClientCoreError::FailedToSetupGateway)
    }
}

pub fn get_client_address_from_stored_keys<T>(
    config: &Config<T>,
) -> Result<Recipient, ClientCoreError>
where
    T: config::NymConfig,
{
    fn load_identity_keys(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<identity::KeyPair, ClientCoreError> {
        let identity_keypair: identity::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_identity_key().to_owned(),
                pathfinder.public_identity_key().to_owned(),
            ))
            .tap_err(|_| log::error!("Failed to read stored identity key files"))?;
        Ok(identity_keypair)
    }

    fn load_sphinx_keys(
        pathfinder: &ClientKeyPathfinder,
    ) -> Result<encryption::KeyPair, ClientCoreError> {
        let sphinx_keypair: encryption::KeyPair =
            pemstore::load_keypair(&pemstore::KeyPairPath::new(
                pathfinder.private_encryption_key().to_owned(),
                pathfinder.public_encryption_key().to_owned(),
            ))
            .tap_err(|_| log::error!("Failed to read stored sphinx key files"))?;
        Ok(sphinx_keypair)
    }

    let pathfinder = ClientKeyPathfinder::new_from_config(config);
    let identity_keypair = load_identity_keys(&pathfinder)?;
    let sphinx_keypair = load_sphinx_keys(&pathfinder)?;

    let client_recipient = Recipient::new(
        *identity_keypair.public_key(),
        *sphinx_keypair.public_key(),
        // TODO: below only works under assumption that gateway address == gateway id
        // (which currently is true)
        NodeIdentity::from_base58_string(config.get_gateway_id())?,
    );

    Ok(client_recipient)
}
