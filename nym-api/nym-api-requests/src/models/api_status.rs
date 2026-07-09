// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signable::{SignableMessageBody, SignedMessage};
use nym_crypto::asymmetric::ed25519;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct ApiHealthResponse {
    pub status: ApiStatus,
    #[serde(default)]
    pub chain_status: ChainStatus,
    pub uptime: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ApiStatus {
    Up,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default, schemars::JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ChainStatus {
    Synced,
    #[default]
    Unknown,
    Stalled {
        #[serde(
            serialize_with = "humantime_serde::serialize",
            deserialize_with = "humantime_serde::deserialize"
        )]
        approximate_amount: Duration,
    },
}

impl ChainStatus {
    pub fn is_synced(&self) -> bool {
        matches!(self, ChainStatus::Synced)
    }
}

impl ApiHealthResponse {
    pub fn new_healthy(uptime: Duration) -> Self {
        ApiHealthResponse {
            status: ApiStatus::Up,
            chain_status: ChainStatus::Synced,
            uptime: uptime.as_secs(),
        }
    }
}

impl ApiStatus {
    pub fn is_up(&self) -> bool {
        matches!(self, ApiStatus::Up)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, schemars::JsonSchema, ToSchema)]
pub struct SignerInformationResponse {
    pub cosmos_address: String,

    pub identity: String,

    pub announce_address: String,

    pub verification_key: Option<String>,
}

// we only allow json and yaml responses so we can easily add additional fields later
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiInformationResponse {
    #[schema(value_type = String)]
    #[serde(with = "ed25519::bs58_ed25519_pubkey")]
    pub identity: ed25519::PublicKey,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyPossessionChallenge {
    pub nonce: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct KeyPossessionChallengePlaintext {
    version: u8,

    #[serde(flatten)]
    challenge: KeyPossessionChallenge,

    purpose: &'static str,
}

impl KeyPossessionChallenge {
    pub fn sign(&self, key: &ed25519::PrivateKey) -> KeyPossessionChallengeResponse {
        self.plaintext_message().sign(key)
    }

    #[allow(clippy::expect_used)]
    pub fn plaintext_message(&self) -> KeyPossessionChallengePlaintext {
        KeyPossessionChallengePlaintext {
            version: 1,
            challenge: *self,
            purpose: "key-possession-challenge",
        }
    }
}

pub type KeyPossessionChallengeResponse = SignedMessage<KeyPossessionChallengePlaintext>;
