// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use crate::node::nym_apis_client::NymApisClient;
use crate::node::nyx_client::NyxClient;
use nym_validator_client::models::KeyRotationDetails;
use tracing::warn;

pub(crate) mod active_keys;
pub(crate) mod controller;
pub(crate) mod key;
pub(crate) mod manager;

/// Reads the key rotation details from the chain, falling back to nym-api if that fails.
pub(crate) async fn get_key_rotation_details(
    nyx_client: &NyxClient,
    nym_apis_client: &NymApisClient,
) -> Result<KeyRotationDetails, NymNodeError> {
    match nyx_client.get_key_rotation_details().await {
        Ok(details) => Ok(details),
        Err(err) => {
            warn!(
                "failed to read the key rotation details from the chain, falling back to nym-api: {err}"
            );
            Ok(nym_apis_client.get_key_rotation_info().await?.details)
        }
    }
}
