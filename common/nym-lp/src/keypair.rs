use std::fmt::{self, Display, Formatter};
use std::ops::Deref;
use std::str::FromStr;

use nym_sphinx::{PrivateKey as SphinxPrivateKey, PublicKey as SphinxPublicKey};
use serde::Serialize;
use utoipa::ToSchema;

use crate::LpError;

pub struct PrivateKey(SphinxPrivateKey);

impl Deref for PrivateKey {
    type Target = SphinxPrivateKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for PrivateKey {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivateKey {
    pub fn new() -> Self {
        let private_key = SphinxPrivateKey::random();
        Self(private_key)
    }

    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.0.to_bytes()).into_string()
    }

    pub fn from_base58_string(s: &str) -> Result<Self, LpError> {
        let bytes: [u8; 32] = bs58::decode(s).into_vec()?.try_into().unwrap();
        Ok(PrivateKey(SphinxPrivateKey::from(bytes)))
    }

    pub fn public_key(&self) -> PublicKey {
        let public_key = SphinxPublicKey::from(&self.0);
        PublicKey(public_key)
    }
}

pub struct PublicKey(SphinxPublicKey);

impl Deref for PublicKey {
    type Target = SphinxPublicKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PublicKey {
    pub fn to_base58_string(&self) -> String {
        bs58::encode(self.0.as_bytes()).into_string()
    }

    pub fn from_base58_string(s: &str) -> Result<Self, LpError> {
        let bytes: [u8; 32] = bs58::decode(s).into_vec()?.try_into().unwrap();
        Ok(PublicKey(SphinxPublicKey::from(bytes)))
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, LpError> {
        Ok(PublicKey(SphinxPublicKey::from(*bytes)))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

impl Default for PublicKey {
    fn default() -> Self {
        let private_key = PrivateKey::default();
        private_key.public_key()
    }
}

pub struct Keypair {
    private_key: PrivateKey,
    public_key: PublicKey,
}

impl Default for Keypair {
    fn default() -> Self {
        Self::new()
    }
}

impl Keypair {
    pub fn new() -> Self {
        let private_key = PrivateKey::default();
        let public_key = private_key.public_key();
        Self {
            private_key,
            public_key,
        }
    }

    pub fn private_key(&self) -> &PrivateKey {
        &self.private_key
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}

impl From<KeypairReadable> for Keypair {
    fn from(keypair: KeypairReadable) -> Self {
        Self {
            private_key: PrivateKey::from_base58_string(&keypair.private).unwrap(),
            public_key: PublicKey::from_base58_string(&keypair.public).unwrap(),
        }
    }
}

impl From<&Keypair> for KeypairReadable {
    fn from(keypair: &Keypair) -> Self {
        Self {
            private: keypair.private_key.to_base58_string(),
            public: keypair.public_key.to_base58_string(),
        }
    }
}
impl FromStr for PrivateKey {
    type Err = LpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        PrivateKey::from_base58_string(s)
    }
}

impl Display for PrivateKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

impl Display for PublicKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58_string())
    }
}

#[derive(Serialize, serde::Deserialize, Clone, ToSchema, Debug)]
pub struct KeypairReadable {
    private: String,
    public: String,
}

impl KeypairReadable {
    pub fn private_key(&self) -> Result<PrivateKey, LpError> {
        PrivateKey::from_base58_string(&self.private)
    }

    pub fn public_key(&self) -> Result<PublicKey, LpError> {
        PublicKey::from_base58_string(&self.public)
    }
}
