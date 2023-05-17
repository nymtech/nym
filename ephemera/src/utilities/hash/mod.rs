use std::fmt::{Debug, Display};

use std::str::FromStr;

use blake2::{Blake2b, Digest};
use digest::consts::U32;
use serde::{Deserialize, Serialize};

pub type Hasher = Blake2bHasher;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn new(hash: [u8; 32]) -> Self {
        Self(hash)
    }

    pub fn inner(&self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn base58(&self) -> String {
        bs58::encode(self.0).into_string()
    }
}

impl FromStr for Hash {
    type Err = bs58::decode::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = bs58::decode(s).into_vec()?;
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&bytes);
        Ok(Self(hash))
    }
}

impl Debug for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.base58())
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.base58())
    }
}

impl From<[u8; 32]> for Hash {
    fn from(hash: [u8; 32]) -> Self {
        Self(hash)
    }
}

/// A trait for hashing data.
pub(crate) trait EphemeraHasher: Default {
    /// Hashes the given data.
    fn digest(data: &[u8]) -> [u8; 32];

    /// Updates the hasher with the given data.
    fn update(&mut self, bytes: &[u8]);

    /// Finalizes the hasher and returns the hash.
    fn finish(&mut self) -> [u8; 32];
}

#[derive(Default)]
pub struct Blake2bHasher {
    hasher: Blake2b<U32>,
}

impl EphemeraHasher for Blake2bHasher {
    fn digest(data: &[u8]) -> [u8; 32] {
        type Blake2b256 = blake2::Blake2b<U32>;
        let mut dest = [0; 32];
        dest.copy_from_slice(Blake2b256::digest(data).as_slice());
        dest
    }

    fn update(&mut self, bytes: &[u8]) {
        self.hasher.update(bytes);
    }

    fn finish(&mut self) -> [u8; 32] {
        self.hasher.finalize_reset().into()
    }
}

pub(crate) trait EphemeraHash {
    fn hash<H: EphemeraHasher>(&self, state: &mut H) -> anyhow::Result<()>;
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::RngCore;

    #[test]
    fn to_base58_parse() {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let hash: Hash = bytes.into();
        let base58 = hash.base58();
        let hash2 = base58.parse::<Hash>().unwrap();
        assert_eq!(hash, hash2);
    }
}
