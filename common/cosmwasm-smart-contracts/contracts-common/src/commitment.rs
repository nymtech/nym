// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "committable_trait")]
pub use digest::{Digest, Output};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
#[cfg(feature = "committable_trait")]
use std::marker::PhantomData;
use std::ops::Deref;

// some sane upper-bound size on commitment sizes
// currently set to 1024bits
pub const MAX_COMMITMENT_SIZE: usize = 128;

// TODO: if we are to use commitments for different types, it might make sense to introduce something like
// CommitmentTypeId field on the below for distinguishing different ones. it would somehow become part of the trait
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, JsonSchema)]
pub struct ContractSafeCommitment(Vec<u8>);

impl Deref for ContractSafeCommitment {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for ContractSafeCommitment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if !self.0.is_empty() {
            write!(f, "0x")?;
        }
        for byte in self.0.iter().take(MAX_COMMITMENT_SIZE) {
            write!(f, "{:02X}", byte)?;
        }
        // just some sanity safeguards
        if self.0.len() > MAX_COMMITMENT_SIZE {
            write!(f, "...")?;
        }
        Ok(())
    }
}

// since cosmwasm stores everything with byte representation of stringified json, it's actually more efficient
// to serialize this as a string as opposed to keeping it as vector of bytes.
// for example vec![255,255] would have string representation of "[255,255]" and will be serialized to
// [91, 50, 53, 53, 44, 50, 53, 53, 93]. the equivalent base58 encoded string `"LUv"` will be serialized to
// [34, 76, 85, 118, 34]
//
// the difference between base58 and base64 is rather minimal and I've gone with base58 for consistency sake
impl Serialize for ContractSafeCommitment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&bs58::encode(&self.0).into_string())
    }
}

impl<'de> Deserialize<'de> for ContractSafeCommitment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = <String>::deserialize(deserializer)?;
        let bytes = bs58::decode(&s)
            .into_vec()
            .map_err(serde::de::Error::custom)?;
        Ok(ContractSafeCommitment(bytes))
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct UnsupportedCommitmentSize;

#[cfg(feature = "committable_trait")]
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct InconsistentCommitmentSize;

#[cfg(feature = "committable_trait")]
pub type DefaultHasher = blake3::Hasher;

#[cfg(feature = "committable_trait")]
pub trait Committable {
    type DigestAlgorithm: Digest;

    fn commitment_size() -> usize {
        <Self::DigestAlgorithm as Digest>::output_size()
    }

    fn to_bytes(&self) -> Vec<u8>;

    fn produce_commitment(&self) -> MessageCommitment<Self> {
        MessageCommitment {
            commitment: Self::DigestAlgorithm::digest(self.to_bytes()),
            _message_type: Default::default(),
        }
    }

    fn verify_commitment(&self, commitment: &MessageCommitment<Self>) -> bool {
        let recomputed = self.produce_commitment();
        recomputed.commitment == commitment.commitment
    }
}

#[cfg(feature = "committable_trait")]
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageCommitment<T>
where
    T: ?Sized + Committable,
{
    commitment: Output<T::DigestAlgorithm>,

    #[serde(skip)]
    _message_type: PhantomData<T>,
}

#[cfg(feature = "committable_trait")]
impl<T> Clone for MessageCommitment<T>
where
    T: ?Sized + Committable,
{
    fn clone(&self) -> Self {
        MessageCommitment {
            commitment: self.commitment.clone(),
            _message_type: Default::default(),
        }
    }
}

#[cfg(feature = "committable_trait")]
impl<T> MessageCommitment<T>
where
    T: ?Sized + Committable,
{
    pub fn value(&self) -> &[u8] {
        self.commitment.as_ref()
    }

    pub fn unchecked_set_value(value: &[u8]) -> Self {
        MessageCommitment {
            commitment: Output::<T::DigestAlgorithm>::clone_from_slice(value),
            _message_type: Default::default(),
        }
    }

    pub fn new(message: &T) -> MessageCommitment<T> {
        message.produce_commitment()
    }

    pub fn contract_safe_commitment(&self) -> ContractSafeCommitment {
        self.into()
    }

    pub fn is_same_as(&self, other: &ContractSafeCommitment) -> bool {
        self.commitment.as_slice() == other.0
    }
}

#[cfg(feature = "committable_trait")]
impl<'a, T> From<&'a MessageCommitment<T>> for ContractSafeCommitment
where
    T: ?Sized + Committable,
{
    fn from(commitment: &'a MessageCommitment<T>) -> Self {
        ContractSafeCommitment(commitment.value().to_vec())
    }
}

#[cfg(feature = "committable_trait")]
impl<'a, T> TryFrom<&'a ContractSafeCommitment> for MessageCommitment<T>
where
    T: ?Sized + Committable,
{
    type Error = InconsistentCommitmentSize;

    fn try_from(value: &'a ContractSafeCommitment) -> Result<Self, Self::Error> {
        if value.len() != <T::DigestAlgorithm as digest::Digest>::output_size() {
            Err(InconsistentCommitmentSize)
        } else {
            Ok(MessageCommitment::unchecked_set_value(value))
        }
    }
}
