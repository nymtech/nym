// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymNodeError;
use crate::node::http::api::api_requests;
use crate::node::http::error::NymNodeHttpError;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_node_requests::api::SignedHostInformation;

pub mod system_info;

pub(crate) fn sign_host_details(
    config: &Config,
    x22519_sphinx: &x25519::PublicKey,
    x25519_noise: &x25519::PublicKey,
    ed22519_identity: &ed25519::KeyPair,
) -> Result<SignedHostInformation, NymNodeError> {
    let x25519_noise = if config.mixnet.debug.unsafe_disable_noise {
        None
    } else {
        Some(*x25519_noise)
    };

    let host_info = api_requests::v1::node::models::HostInformation {
        ip_address: config.host.public_ips.clone(),
        hostname: config.host.hostname.clone(),
        keys: api_requests::v1::node::models::HostKeys {
            ed25519_identity: *ed22519_identity.public_key(),
            x25519_sphinx: *x22519_sphinx,
            x25519_noise,
        },
    };

    let signed_info = SignedHostInformation::new(host_info, ed22519_identity.private_key())
        .map_err(NymNodeHttpError::from)?;
    Ok(signed_info)
}
