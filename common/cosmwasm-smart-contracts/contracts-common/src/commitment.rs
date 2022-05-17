// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "committable_trait")]
pub use digest::{Digest, Output};
#[cfg(feature = "committable_trait")]
use std::marker::PhantomData;

// some sane upper-bound size on commitment sizes
// currently set to 1024bits
pub const MAX_COMMITMENT_SIZE: usize = 128;

// TODO: if we are to use commitments for different types, it might make sense to introduce something like
// CommitmentTypeId field on the below for distinguishing different ones. it would somehow become part of the trait
pub type ContractSafeCommitment = Vec<u8>;

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
pub struct MessageCommitment<T>
where
    T: ?Sized + Committable,
{
    commitment: Output<T::DigestAlgorithm>,
    _message_type: PhantomData<*const T>,
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
}

#[cfg(feature = "committable_trait")]
impl<'a, T> From<&'a MessageCommitment<T>> for ContractSafeCommitment
where
    T: ?Sized + Committable,
{
    fn from(commitment: &'a MessageCommitment<T>) -> Self {
        ContractSafeCommitment::from(commitment.value())
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
