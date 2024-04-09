// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::api::v1::node::models::{HostInformation, LegacyHostInformation};
use crate::error::Error;
use nym_crypto::asymmetric::identity;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::Deref;

#[cfg(feature = "client")]
pub mod client;
pub mod v1;

#[cfg(feature = "client")]
pub use client::Client;

// create the type alias manually if openapi is not enabled
#[cfg(not(feature = "openapi"))]
pub type SignedHostInformation = SignedData<HostInformation>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", aliases(SignedHostInformation = SignedData<HostInformation>))]
pub struct SignedData<T> {
    // #[serde(flatten)]
    pub data: T,
    pub signature: String,
}

impl<T> SignedData<T> {
    pub fn new(data: T, key: &identity::PrivateKey) -> Result<Self, Error>
    where
        T: Serialize,
    {
        let plaintext = serde_json::to_string(&data)?;
        let signature = key.sign(plaintext).to_base58_string();
        Ok(SignedData { data, signature })
    }

    pub fn verify(&self, key: &identity::PublicKey) -> bool
    where
        T: Serialize,
    {
        let Ok(plaintext) = serde_json::to_string(&self.data) else {
            return false;
        };
        let Ok(signature) = identity::Signature::from_base58_string(&self.signature) else {
            return false;
        };

        key.verify(plaintext, &signature).is_ok()
    }
}

impl SignedHostInformation {
    pub fn verify_host_information(&self) -> bool {
        let Ok(pub_key) = identity::PublicKey::from_base58_string(&self.keys.ed25519_identity)
        else {
            return false;
        };

        if self.verify(&pub_key) {
            return true;
        }

        // attempt to verify legacy signature
        SignedData {
            data: LegacyHostInformation::from(self.data.clone()),
            signature: self.signature.clone(),
        }
        .verify(&pub_key)
    }
}

impl<T> Deref for SignedData<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct ErrorResponse {
    pub message: String,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.message.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::{ed25519, x25519};
    use rand_chacha::rand_core::SeedableRng;
    use std::net::IpAddr;

    #[derive(Serialize, Deserialize)]
    struct LegacyHostInformation {
        pub ip_address: Vec<IpAddr>,
        pub hostname: Option<String>,
        pub keys: LegacyKeys,
    }

    #[derive(Serialize, Deserialize)]
    struct LegacyKeys {
        pub ed25519: String,
        pub x25519: String,
    }

    #[test]
    fn dummy_signed_host_verification() {
        let mut rng = rand_chacha::ChaCha20Rng::from_seed([0u8; 32]);
        let ed22519 = ed25519::KeyPair::new(&mut rng);
        let x25519_sphinx = x25519::KeyPair::new(&mut rng);

        let host_info = crate::api::v1::node::models::HostInformation {
            ip_address: vec!["1.1.1.1".parse().unwrap()],
            hostname: Some("foomp.com".to_string()),
            keys: crate::api::v1::node::models::HostKeys {
                ed25519_identity: ed22519.public_key().to_base58_string(),
                x25519_sphinx: x25519_sphinx.public_key().to_base58_string(),
                x25519_noise: "".to_string(),
            },
        };

        let signed_info = SignedHostInformation::new(host_info, ed22519.private_key()).unwrap();
        assert!(signed_info.verify(ed22519.public_key()));
        assert!(signed_info.verify_host_information())
    }

    #[test]
    fn dummy_legacy_signed_host_verification() {
        let mut rng = rand_chacha::ChaCha20Rng::from_seed([0u8; 32]);
        let ed22519 = ed25519::KeyPair::new(&mut rng);
        let x25519_sphinx = x25519::KeyPair::new(&mut rng);

        let legacy_info = crate::api::v1::node::models::LegacyHostInformation {
            ip_address: vec!["1.1.1.1".parse().unwrap()],
            hostname: Some("foomp.com".to_string()),
            keys: crate::api::v1::node::models::LegacyHostKeys {
                ed25519: ed22519.public_key().to_base58_string(),
                x25519: x25519_sphinx.public_key().to_base58_string(),
            },
        };

        let host_info = crate::api::v1::node::models::HostInformation {
            ip_address: legacy_info.ip_address.clone(),
            hostname: legacy_info.hostname.clone(),
            keys: crate::api::v1::node::models::HostKeys {
                ed25519_identity: legacy_info.keys.ed25519.clone(),
                x25519_sphinx: legacy_info.keys.x25519.clone(),
                x25519_noise: "".to_string(),
            },
        };

        // signature on legacy data
        let signature = SignedData::new(legacy_info, ed22519.private_key())
            .unwrap()
            .signature;

        // signed blob with the 'current' structure
        let current_struct = SignedData {
            data: host_info,
            signature,
        };

        assert!(!current_struct.verify(ed22519.public_key()));
        assert!(current_struct.verify_host_information())
    }
}
