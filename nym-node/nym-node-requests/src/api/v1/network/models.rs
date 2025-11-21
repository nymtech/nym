// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::ed25519;
use nym_crypto::asymmetric::ed25519::serde_helpers::bs58_ed25519_pubkey;
use nym_upgrade_mode_check::UpgradeModeAttestation;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UpgradeModeStatus {
    pub enabled: bool,

    #[serde(with = "time::serde::rfc3339")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub last_queried: OffsetDateTime,

    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub attestation_provider: Url,

    #[serde(with = "bs58_ed25519_pubkey")]
    #[cfg_attr(feature = "openapi", schema(value_type = String))]
    pub attester_pubkey: ed25519::PublicKey,

    pub published_attestation: Option<UpgradeModeAttestation>,
}
