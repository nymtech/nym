use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    net::SocketAddr,
    ops::Deref,
    str::FromStr,
};

use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use nym_crypto::asymmetric::encryption::PrivateKey;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::StaticSecret;

use crate::error::GatewayError;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) enum ClientMessage {
    Init(InitMessage),
    Final(Client),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct InitMessage {
    pub_key: ClientPublicKey,
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Client {
    // base64 encoded public key, using x25519-dalek for impl
    pub(crate) pub_key: ClientPublicKey,
    pub(crate) socket: SocketAddr,
    pub(crate) mac: ClientMac,
}

pub type HmacSha256 = Hmac<Sha256>;

impl Client {
    // Reusable secret should be gateways Wireguard PK
    // Client should perform this step when generating its payload, using its own WG PK
    pub fn verify(&self, gateway_key: &PrivateKey, nonce: u64) -> Result<(), GatewayError> {
        #[allow(clippy::expect_used)]
        let static_secret =
            StaticSecret::try_from(gateway_key.to_bytes()).expect("This is infalliable");
        let dh = static_secret.diffie_hellman(&self.pub_key);
        let mut mac = HmacSha256::new_from_slice(dh.as_bytes())?;
        mac.update(self.pub_key.as_bytes());
        mac.update(self.socket.ip().to_string().as_bytes());
        mac.update(self.socket.port().to_string().as_bytes());
        mac.update(&nonce.to_le_bytes());
        Ok(mac.verify_slice(&self.mac)?)
    }

    pub fn pub_key(&self) -> &ClientPublicKey {
        &self.pub_key
    }

    pub fn socket(&self) -> SocketAddr {
        self.socket
    }
}

// This should go into nym-wireguard crate
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClientPublicKey(x25519_dalek::PublicKey);
#[derive(Debug, Clone)]
pub(crate) struct ClientMac(Vec<u8>);

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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mac_bytes: Vec<u8> = general_purpose::STANDARD
            .decode(s)
            .map_err(|_| "Could not base64 decode public key mac representation".to_string())?;
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key_bytes: [u8; 32] = general_purpose::STANDARD
            .decode(s)
            .map_err(|_| "Could not base64 decode public key representation".to_string())?
            .try_into()
            .map_err(|_| "Invalid key length".to_string())?;
        Ok(ClientPublicKey(x25519_dalek::PublicKey::from(key_bytes)))
    }
}

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

pub(crate) type ClientRegistry = HashMap<SocketAddr, Client>;
