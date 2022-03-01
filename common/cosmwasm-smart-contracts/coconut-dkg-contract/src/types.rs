// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

// encoded in base58
pub type EncodedChannelPublicKey = String;

pub type NodeIndex = u64;

#[derive(Serialize, Deserialize)]
pub struct IssuerDetails {
    pub public_key: EncodedChannelPublicKey,
    pub node_index: NodeIndex,
}

impl IssuerDetails {
    pub fn new(public_key: EncodedChannelPublicKey, node_index: NodeIndex) -> Self {
        IssuerDetails {
            public_key,
            node_index,
        }
    }
}

// another experiment:
pub struct ReceivedShare {
    node_index: NodeIndex,
    ciphertext: Vec<()>, // or maybe just concrete type? I guess that will depend on encryption scheme
}

pub struct IssuerDetailsResponse {
    pub details: Option<IssuerDetails>,
}

pub struct InactiveIssuerDetailsResponse {
    pub details: Option<IssuerDetails>,
    pub last_seen: Option<u64>,
}
