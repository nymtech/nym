// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials_interface::CredentialSpendingData;
use nym_wireguard_types::PeerPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TopUpMessage {
    /// Base64 encoded x25519 public key
    pub pub_key: PeerPublicKey,

    /// Ecash credential
    pub credential: Option<CredentialSpendingData>,
}
