// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::signing::{MessageSignature, SignableMessage, SigningAlgorithm};
use cosmwasm_std::{Api, StdError, VerificationError};
use serde::Serialize;
use thiserror::Error;

pub trait Verifier {
    type Error: From<StdError>;

    fn verify_message<T: Serialize>(
        &self,
        message: SignableMessage<T>,
        signature: MessageSignature,
        public_key: &[u8],
    ) -> Result<bool, Self::Error> {
        match message.algorithm {
            SigningAlgorithm::Ed25519 => {
                let plaintext = message.to_plaintext()?;
                self.verify_ed25519(&plaintext, signature.as_ref(), public_key)
            }
            SigningAlgorithm::Secp256k1 => {
                let plaintext = message.to_sha256_plaintext_digest()?;
                self.verify_secp256k1(&plaintext, signature.as_ref(), public_key)
            }
        }
    }

    fn verify_ed25519(
        &self,
        _message: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, Self::Error> {
        unimplemented!()
    }

    fn verify_secp256k1(
        &self,
        _message_hash: &[u8],
        _signature: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, Self::Error> {
        unimplemented!()
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum ApiVerifierError {
    #[error(transparent)]
    Verification(#[from] VerificationError),

    #[error(transparent)]
    Std(#[from] StdError),
}

impl<T> Verifier for T
where
    T: Api + ?Sized,
{
    type Error = ApiVerifierError;

    fn verify_ed25519(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Self::Error> {
        Ok(self.ed25519_verify(message, signature, public_key)?)
    }

    fn verify_secp256k1(
        &self,
        message_hash: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool, Self::Error> {
        Ok(self.secp256k1_verify(message_hash, signature, public_key)?)
    }
}
