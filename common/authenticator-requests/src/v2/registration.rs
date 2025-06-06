// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use base64::{engine::general_purpose, Engine};
use nym_credentials_interface::CredentialSpendingData;
use nym_wireguard_types::PeerPublicKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::time::SystemTime;
use std::{fmt, ops::Deref, str::FromStr};

#[cfg(feature = "verify")]
use hmac::{Hmac, Mac};
#[cfg(feature = "verify")]
use nym_crypto::asymmetric::x25519::{PrivateKey, PublicKey};
#[cfg(feature = "verify")]
use sha2::Sha256;

pub type PendingRegistrations = HashMap<PeerPublicKey, RegistrationData>;
pub type PrivateIPs = HashMap<IpAddr, Taken>;

#[cfg(feature = "verify")]
pub type HmacSha256 = Hmac<Sha256>;

pub type Nonce = u64;
pub type Taken = Option<SystemTime>;

pub const BANDWIDTH_CAP_PER_DAY: u64 = 1024 * 1024 * 1024; // 1 GB

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct InitMessage {
    /// Base64 encoded x25519 public key
    pub pub_key: PeerPublicKey,
}

impl InitMessage {
    pub fn new(pub_key: PeerPublicKey) -> Self {
        InitMessage { pub_key }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FinalMessage {
    /// Gateway client data
    pub gateway_client: GatewayClient,

    /// Ecash credential
    pub credential: Option<CredentialSpendingData>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RegistrationData {
    pub nonce: u64,
    pub gateway_data: GatewayClient,
    pub wg_port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RegistredData {
    pub pub_key: PeerPublicKey,
    pub private_ip: IpAddr,
    pub wg_port: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RemainingBandwidthData {
    pub available_bandwidth: i64,
}

/// Client that wants to register sends its PublicKey bytes mac digest encrypted with a DH shared secret.
/// Gateway/Nym node can then verify pub_key payload using the same process
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GatewayClient {
    /// Base64 encoded x25519 public key
    pub pub_key: PeerPublicKey,

    /// Assigned private IP
    pub private_ip: IpAddr,

    /// Sha256 hmac on the data (alongside the prior nonce)
    pub mac: ClientMac,
}

impl GatewayClient {
    #[cfg(feature = "verify")]
    pub fn new(
        local_secret: &PrivateKey,
        remote_public: x25519_dalek::PublicKey,
        private_ip: IpAddr,
        nonce: u64,
    ) -> Self {
        let local_public = PublicKey::from(local_secret);
        let remote_public = PublicKey::from(remote_public);

        let dh = local_secret.diffie_hellman(&remote_public);

        // TODO: change that to use our nym_crypto::hmac module instead
        #[allow(clippy::expect_used)]
        let mut mac = HmacSha256::new_from_slice(&dh[..])
            .expect("x25519 shared secret is always 32 bytes long");

        mac.update(local_public.as_bytes());
        mac.update(private_ip.to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());

        GatewayClient {
            pub_key: PeerPublicKey::new(local_public.into()),
            private_ip,
            mac: ClientMac(mac.finalize().into_bytes().to_vec()),
        }
    }

    // Reusable secret should be gateways Wireguard PK
    // Client should perform this step when generating its payload, using its own WG PK
    #[cfg(feature = "verify")]
    pub fn verify(&self, gateway_key: &PrivateKey, nonce: u64) -> Result<(), Error> {
        // use gateways key as a ref to an x25519_dalek key
        let dh = gateway_key.inner().diffie_hellman(&self.pub_key);

        // TODO: change that to use our nym_crypto::hmac module instead
        #[allow(clippy::expect_used)]
        let mut mac = HmacSha256::new_from_slice(dh.as_bytes())
            .expect("x25519 shared secret is always 32 bytes long");

        mac.update(self.pub_key.as_bytes());
        mac.update(self.private_ip.to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());

        mac.verify_slice(&self.mac)
            .map_err(|source| Error::FailedClientMacVerification {
                client: self.pub_key.to_string(),
                source,
            })
    }

    pub fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

// TODO: change the inner type into generic array of size HmacSha256::OutputSize
// TODO2: rely on our internal crypto/hmac
#[derive(Debug, Clone, PartialEq)]
pub struct ClientMac(Vec<u8>);

impl fmt::Display for ClientMac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(&self.0))
    }
}

impl ClientMac {
    #[allow(dead_code)]
    pub fn new(mac: Vec<u8>) -> Self {
        ClientMac(mac)
    }
}

impl Deref for ClientMac {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ClientMac {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mac_bytes: Vec<u8> =
            general_purpose::STANDARD
                .decode(s)
                .map_err(|source| Error::MalformedClientMac {
                    mac: s.to_string(),
                    source,
                })?;

        Ok(ClientMac(mac_bytes))
    }
}

impl Serialize for ClientMac {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(self.0.clone());
        serializer.serialize_str(&encoded_key)
    }
}

impl<'de> Deserialize<'de> for ClientMac {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded_key = String::deserialize(deserializer)?;
        ClientMac::from_str(&encoded_key).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_crypto::asymmetric::x25519;

    #[test]
    #[cfg(feature = "verify")]
    fn client_request_roundtrip() {
        let mut rng = rand::thread_rng();

        let gateway_key_pair = x25519::KeyPair::new(&mut rng);
        let client_key_pair = x25519::KeyPair::new(&mut rng);

        let nonce = 1234567890;

        let client = GatewayClient::new(
            client_key_pair.private_key(),
            x25519_dalek::PublicKey::from(gateway_key_pair.public_key().to_bytes()),
            "10.0.0.42".parse().unwrap(),
            nonce,
        );
        assert!(client.verify(gateway_key_pair.private_key(), nonce).is_ok())
    }
}
