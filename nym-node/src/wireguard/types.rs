// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::wireguard::error::WireguardError;
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use nym_crypto::asymmetric::encryption::PrivateKey;
pub(crate) use nym_node_requests::api::v1::gateway::client_interefaces::wireguard::models::{
    Client as ClientRequest, ClientMessage as ClientMessageRequest,
    InitMessage as InitMessageRequest,
};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::ops::Deref;
use std::str::FromStr;
use x25519_dalek::StaticSecret;

pub type HmacSha256 = Hmac<Sha256>;
pub type ClientRegistry = HashMap<SocketAddr, Client>;
pub type Nonce = u64;
pub type PendingRegistrations = HashMap<ClientPublicKey, Nonce>;

#[derive(Debug, Clone)]
pub(crate) enum ClientMessage {
    Init(InitMessage),
    Final(Client),
}

impl TryFrom<ClientMessageRequest> for ClientMessage {
    type Error = WireguardError;

    fn try_from(value: ClientMessageRequest) -> Result<Self, Self::Error> {
        match value {
            ClientMessageRequest::Initial(init) => init.try_into().map(ClientMessage::Init),
            ClientMessageRequest::Final(final_msg) => {
                final_msg.try_into().map(ClientMessage::Final)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct InitMessage {
    pub pub_key: ClientPublicKey,
}

impl TryFrom<InitMessageRequest> for InitMessage {
    type Error = WireguardError;

    fn try_from(value: InitMessageRequest) -> Result<Self, Self::Error> {
        Ok(InitMessage {
            pub_key: value.pub_key.parse()?,
        })
    }
}

impl InitMessage {
    pub fn pub_key(&self) -> &ClientPublicKey {
        &self.pub_key
    }

    #[allow(dead_code)]
    pub fn new(pub_key: ClientPublicKey) -> Self {
        InitMessage { pub_key }
    }
}

// Client that wants to register sends its PublicKey and SocketAddr bytes mac digest encrypted with a DH shared secret.
// Gateway can then verify pub_key payload using the sme process
#[derive(Debug, Clone)]
// TEMP:
#[derive(Serialize)]
pub struct Client {
    // base64 encoded public key, using x25519-dalek for impl
    pub pub_key: ClientPublicKey,
    pub socket: SocketAddr,
    pub mac: ClientMac,
}

impl TryFrom<ClientRequest> for Client {
    type Error = WireguardError;

    fn try_from(value: ClientRequest) -> Result<Self, Self::Error> {
        Ok(Client {
            pub_key: value.pub_key.parse()?,
            socket: value.socket.parse().map_err(|source| {
                WireguardError::MalformedClientSocketAddress {
                    raw: value.socket,
                    source,
                }
            })?,
            mac: value.mac.parse()?,
        })
    }
}

impl Client {
    // Reusable secret should be gateways Wireguard PK
    // Client should perform this step when generating its payload, using its own WG PK
    pub fn verify(&self, gateway_key: &PrivateKey, nonce: u64) -> Result<(), WireguardError> {
        // convert from 1.0 x25519-dalek private key into 2.0 x25519-dalek
        #[allow(clippy::expect_used)]
        let static_secret = StaticSecret::try_from(gateway_key.to_bytes())
            .expect("conversion between x25519 private keys is infallible");

        let dh = static_secret.diffie_hellman(&self.pub_key);

        // TODO: change that to use our nym_crypto::hmac module instead

        #[allow(clippy::expect_used)]
        let mut mac = HmacSha256::new_from_slice(dh.as_bytes())
            .expect("x25519 shared secret is always 32 bytes long");

        mac.update(self.pub_key.as_bytes());
        mac.update(self.socket.ip().to_string().as_bytes());
        mac.update(self.socket.port().to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());

        mac.verify_slice(&self.mac)
            .map_err(|source| WireguardError::FailedClientMacVerification {
                client: self.pub_key.to_string(),
                source,
            })
    }

    pub fn pub_key(&self) -> ClientPublicKey {
        self.pub_key
    }

    pub fn socket(&self) -> SocketAddr {
        self.socket
    }
}

// This should go into nym-wireguard crate
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ClientPublicKey(x25519_dalek::PublicKey);

// TODO: change the inner type into generic array of size HmacSha256::OutputSize
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

impl fmt::Display for ClientPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(self.0.as_bytes()))
    }
}

impl Hash for ClientPublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state)
    }
}

impl FromStr for ClientMac {
    type Err = WireguardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mac_bytes: Vec<u8> = general_purpose::STANDARD.decode(s).map_err(|source| {
            WireguardError::MalformedClientMac {
                mac: s.to_string(),
                source,
            }
        })?;

        Ok(ClientMac(mac_bytes))
    }
}

impl ClientPublicKey {
    #[allow(dead_code)]
    pub fn new(key: x25519_dalek::PublicKey) -> Self {
        ClientPublicKey(key)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Deref for ClientPublicKey {
    type Target = x25519_dalek::PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for ClientPublicKey {
    type Err = WireguardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key_bytes: Vec<u8> = general_purpose::STANDARD.decode(s).map_err(|source| {
            WireguardError::MalformedClientPublicKeyEncoding {
                pub_key: s.to_string(),
                source,
            }
        })?;

        let decoded_length = key_bytes.len();
        let Ok(key_arr): Result<[u8; 32], _> = key_bytes.try_into() else {
            return Err(WireguardError::InvalidClientPublicKeyLength {
                pub_key: s.to_string(),
                decoded_length,
            })?;
        };

        Ok(ClientPublicKey(x25519_dalek::PublicKey::from(key_arr)))
    }
}

// TEMPORARY:

impl Serialize for ClientPublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(self.0.as_bytes());
        serializer.serialize_str(&encoded_key)
    }
}

impl<'de> Deserialize<'de> for ClientPublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded_key = String::deserialize(deserializer)?;
        Ok(ClientPublicKey::from_str(&encoded_key).map_err(serde::de::Error::custom))?
    }
}

impl Serialize for ClientMac {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(&self.0);
        serializer.serialize_str(&encoded_key)
    }
}
