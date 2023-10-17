use std::{
    fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    str::FromStr,
};

use base64::{engine::general_purpose, Engine};
use boringtun::x25519::PublicKey;
use serde::Serialize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PeerPublicKey(PublicKey);

impl fmt::Display for PeerPublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", general_purpose::STANDARD.encode(self.0.as_bytes()))
    }
}

impl Hash for PeerPublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_bytes().hash(state)
    }
}

impl PeerPublicKey {
    #[allow(dead_code)]
    pub fn new(key: PublicKey) -> Self {
        PeerPublicKey(key)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl Deref for PeerPublicKey {
    type Target = PublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromStr for PeerPublicKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let key_bytes: [u8; 32] = general_purpose::STANDARD
            .decode(s)
            .map_err(|_| "Could not base64 decode public key representation".to_string())?
            .try_into()
            .map_err(|_| "Invalid key length".to_string())?;
        Ok(PeerPublicKey(PublicKey::from(key_bytes)))
    }
}

impl Serialize for PeerPublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let encoded_key = general_purpose::STANDARD.encode(self.0.as_bytes());
        serializer.serialize_str(&encoded_key)
    }
}

impl<'de> serde::Deserialize<'de> for PeerPublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let encoded_key = String::deserialize(deserializer)?;
        Ok(PeerPublicKey::from_str(&encoded_key).map_err(serde::de::Error::custom))?
    }
}
