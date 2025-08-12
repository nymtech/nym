// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::UpgradeModeCheckError;
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::generate_user_agent;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub struct UpgradeModeAttestation {
    #[serde(flatten)]
    pub content: UpgradeModeAttestationContent,

    #[serde(with = "ed25519::bs58_ed25519_signature")]
    pub signature: ed25519::Signature,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(tag = "type")]
#[serde(rename = "upgrade_mode")]
pub struct UpgradeModeAttestationContent {
    #[serde(with = "time::serde::timestamp")]
    pub starting_time: OffsetDateTime,

    #[serde(with = "ed25519::bs58_ed25519_pubkey")]
    pub attester_public_key: ed25519::PublicKey,
}

impl UpgradeModeAttestation {
    pub fn verify(&self) -> bool {
        self.content
            .attester_public_key
            .verify(self.content.as_json(), &self.signature)
            .is_ok()
    }
}

impl UpgradeModeAttestationContent {
    pub fn as_json(&self) -> String {
        // SAFETY: Serialize impl is valid and we have no non-string map keys
        #[allow(clippy::unwrap_used)]
        serde_json::to_string(&self).unwrap()
    }
}

pub fn generate_new_attestation(key: &ed25519::PrivateKey) -> UpgradeModeAttestation {
    generate_new_attestation_with_starting_time(key, OffsetDateTime::now_utc())
}

pub fn generate_new_attestation_with_starting_time(
    key: &ed25519::PrivateKey,
    starting_time: OffsetDateTime,
) -> UpgradeModeAttestation {
    let content = UpgradeModeAttestationContent {
        starting_time,
        attester_public_key: key.into(),
    };
    UpgradeModeAttestation {
        signature: key.sign(content.as_json()),
        content,
    }
}

pub async fn attempt_retrieve(
    url: &str,
) -> Result<Option<UpgradeModeAttestation>, UpgradeModeCheckError> {
    let retrieval_failure = |source| UpgradeModeCheckError::AttestationRetrievalFailure {
        url: url.to_string(),
        source,
    };

    let attestation = reqwest::ClientBuilder::new()
        .user_agent(generate_user_agent!())
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(retrieval_failure)?
        .get(url)
        .send()
        .await
        .map_err(retrieval_failure)?
        .json::<Option<UpgradeModeAttestation>>()
        .await
        .map_err(retrieval_failure)?;

    Ok(attestation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upgrade_mode_attestation_serde_json() -> anyhow::Result<()> {
        // unix timestamp: 1629720000
        let starting_time = time::macros::datetime!(2021-08-23 12:00 UTC);

        let key = ed25519::PrivateKey::from_bytes(&[
            108, 49, 193, 21, 126, 161, 249, 85, 242, 207, 74, 195, 238, 6, 64, 149, 201, 140, 248,
            163, 122, 170, 79, 198, 87, 85, 36, 29, 243, 92, 64, 161,
        ])?;

        let attestation = generate_new_attestation_with_starting_time(&key, starting_time);

        let attestation_json = serde_json::to_string(&attestation)?;
        let attestation_content_json = attestation.content.as_json();

        let expected_attestation = r#"{"type":"upgrade_mode","starting_time":1629720000,"attester_public_key":"3pkFcBXCEmbmXBT2G8CkFMuKisJcH54mbBGvncHaDibt","signature":"5rWUr2ypaDTtrMKegMP3tQkkZGFAuhNTnEVCVe5Azv6QqvLzoGdQiMkFmeyhDd1XSfoXpL9fFM58rsdA1kf4GYMM"}"#;
        let expected_content = r#"{"type":"upgrade_mode","starting_time":1629720000,"attester_public_key":"3pkFcBXCEmbmXBT2G8CkFMuKisJcH54mbBGvncHaDibt"}"#;

        assert_eq!(attestation_content_json, expected_content);
        assert_eq!(attestation_json, expected_attestation);

        let recovered_attestation = serde_json::from_str(&attestation_json)?;
        assert_eq!(attestation, recovered_attestation);

        let recovered_content = serde_json::from_str(&attestation_content_json)?;
        assert_eq!(attestation.content, recovered_content);

        Ok(())
    }
}
