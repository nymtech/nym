// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use crate::PeerPublicKey;
use base64::{engine::general_purpose, Engine};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::{fmt, ops::Deref, str::FromStr};

#[cfg(feature = "verify")]
use hmac::{Hmac, Mac};
#[cfg(feature = "verify")]
use nym_crypto::asymmetric::encryption::PrivateKey;
#[cfg(feature = "verify")]
use sha2::Sha256;

pub type GatewayClientRegistry = DashMap<PeerPublicKey, GatewayClient>;
pub type PendingRegistrations = DashMap<PeerPublicKey, RegistrationData>;
pub type PrivateIPs = DashMap<IpAddr, Free>;

#[cfg(feature = "verify")]
pub type HmacSha256 = Hmac<Sha256>;

pub type Nonce = u64;
pub type Free = bool;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum ClientMessage {
    Initial(InitMessage),
    Final(GatewayClient),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct InitMessage {
    /// Base64 encoded x25519 public key
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = Byte))]
    pub pub_key: PeerPublicKey,
}

impl InitMessage {
    pub fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    pub fn new(pub_key: PeerPublicKey) -> Self {
        InitMessage { pub_key }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RegistrationData {
    pub nonce: u64,
    pub gateway_data: GatewayClient,
    pub wg_port: u16,
}

/// Client that wants to register sends its PublicKey bytes mac digest encrypted with a DH shared secret.
/// Gateway/Nym node can then verify pub_key payload using the same process
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct GatewayClient {
    /// Base64 encoded x25519 public key
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = Byte))]
    pub pub_key: PeerPublicKey,

    /// Assigned private IP
    pub private_ip: IpAddr,

    /// Sha256 hmac on the data (alongside the prior nonce)
    #[cfg_attr(feature = "openapi", schema(value_type = String, format = Byte))]
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
        // convert from 1.0 x25519-dalek private key into 2.0 x25519-dalek
        #[allow(clippy::expect_used)]
        let static_secret = x25519_dalek::StaticSecret::from(local_secret.to_bytes());
        let local_public: x25519_dalek::PublicKey = (&static_secret).into();

        let dh = static_secret.diffie_hellman(&remote_public);

        // TODO: change that to use our nym_crypto::hmac module instead
        #[allow(clippy::expect_used)]
        let mut mac = HmacSha256::new_from_slice(dh.as_bytes())
            .expect("x25519 shared secret is always 32 bytes long");

        mac.update(local_public.as_bytes());
        mac.update(private_ip.to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());

        GatewayClient {
            pub_key: PeerPublicKey::new(local_public),
            private_ip,
            mac: ClientMac(mac.finalize().into_bytes().to_vec()),
        }
    }

    // Reusable secret should be gateways Wireguard PK
    // Client should perform this step when generating its payload, using its own WG PK
    #[cfg(feature = "verify")]
    pub fn verify(&self, gateway_key: &PrivateKey, nonce: u64) -> Result<(), Error> {
        // convert from 1.0 x25519-dalek private key into 2.0 x25519-dalek
        #[allow(clippy::expect_used)]
        let static_secret = x25519_dalek::StaticSecret::from(gateway_key.to_bytes());

        let dh = static_secret.diffie_hellman(&self.pub_key);

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
#[derive(Debug, Clone)]
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
    use nym_crypto::asymmetric::encryption;

    #[test]
    #[cfg(feature = "verify")]
    fn client_request_roundtrip() {
        let mut rng = rand::thread_rng();

        let gateway_key_pair = encryption::KeyPair::new(&mut rng);
        let client_key_pair = encryption::KeyPair::new(&mut rng);

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
