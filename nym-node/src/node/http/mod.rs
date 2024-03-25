// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_crypto::asymmetric::{encryption, identity};
use nym_node::config::Config;
use nym_node::error::NymNodeError;
use nym_node_http_api::api::api_requests;
use nym_node_http_api::api::api_requests::SignedHostInformation;
use nym_node_http_api::NymNodeHttpError;

pub(crate) mod system_info;

pub(crate) fn sign_host_details(
    config: &Config,
    x22519_sphinx: &encryption::PublicKey,
    ed22519_identity: &identity::KeyPair,
) -> Result<api_requests::v1::node::models::SignedHostInformation, NymNodeError> {
    let host_info = api_requests::v1::node::models::HostInformation {
        ip_address: config.host.public_ips.clone(),
        hostname: config.host.hostname.clone(),
        keys: api_requests::v1::node::models::HostKeys {
            ed25519: ed22519_identity.public_key().to_base58_string(),
            x25519: x22519_sphinx.to_base58_string(),
        },
    };

    let signed_info = SignedHostInformation::new(host_info, ed22519_identity.private_key())
        .map_err(NymNodeHttpError::from)?;
    Ok(signed_info)
}

// pub(crate) fn run_http_api(config: &Config, task_client: TaskClient)
