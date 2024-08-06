// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::dkg::state::serde_helpers::bte_pk_serde;
use cosmwasm_std::Addr;
use nym_coconut_dkg_common::dealer::DealerDetails;
use nym_coconut_dkg_common::types::EncodedBTEPublicKeyWithProof;
use nym_dkg::bte::PublicKeyWithProof;
use nym_dkg::{bte, NodeIndex};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Deserialize, Debug, Serialize)]
pub(crate) enum ParticipantState {
    Invalid(KeyRejectionReason),
    VerifiedKey(#[serde(with = "bte_pk_serde")] Box<PublicKeyWithProof>),
}

impl ParticipantState {
    pub fn is_valid(&self) -> bool {
        matches!(self, ParticipantState::VerifiedKey(..))
    }

    pub fn public_key(&self) -> Option<bte::PublicKey> {
        match self {
            ParticipantState::Invalid(_) => None,
            ParticipantState::VerifiedKey(key_with_proof) => Some(*key_with_proof.public_key()),
        }
    }

    fn from_raw_encoded_key(raw: EncodedBTEPublicKeyWithProof) -> Self {
        let bytes = match bs58::decode(raw).into_vec() {
            Ok(bytes) => bytes,
            Err(err) => {
                return ParticipantState::Invalid(
                    KeyRejectionReason::MalformedBTEPublicKeyEncoding {
                        err_msg: err.to_string(),
                    },
                );
            }
        };

        let key = match PublicKeyWithProof::try_from_bytes(&bytes) {
            Ok(key) => key,
            Err(err) => {
                return ParticipantState::Invalid(KeyRejectionReason::MalformedBTEPublicKey {
                    err_msg: err.to_string(),
                });
            }
        };

        if !key.verify() {
            return ParticipantState::Invalid(KeyRejectionReason::InvalidBTEPublicKey);
        }

        ParticipantState::VerifiedKey(Box::new(key))
    }
}

// note: each dealer is also a receiver which simplifies some logic significantly
#[derive(Clone, Deserialize, Debug, Serialize)]
pub(crate) struct DkgParticipant {
    pub(crate) address: Addr,
    pub(crate) assigned_index: NodeIndex,
    pub(crate) state: ParticipantState,
}

impl From<DealerDetails> for DkgParticipant {
    fn from(dealer: DealerDetails) -> Self {
        DkgParticipant {
            address: dealer.address,
            state: ParticipantState::from_raw_encoded_key(dealer.bte_public_key_with_proof),
            assigned_index: dealer.assigned_index,
        }
    }
}

impl DkgParticipant {
    #[cfg(test)]
    pub(crate) fn unwrap_key(&self) -> PublicKeyWithProof {
        if let ParticipantState::VerifiedKey(key) = &self.state {
            return *key.clone();
        }
        panic!("no key")
    }

    #[cfg(test)]
    pub fn unwrap_rejection(&self) -> KeyRejectionReason {
        if let ParticipantState::Invalid(rejection) = &self.state {
            return rejection.clone();
        }
        panic!("not rejected")
    }
}

#[derive(Clone, Error, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum KeyRejectionReason {
    #[error("provided BTE Public key encoding is malformed: {err_msg}")]
    MalformedBTEPublicKeyEncoding { err_msg: String },

    #[error("provided BTE Public key has invalid byte representation: {err_msg}")]
    MalformedBTEPublicKey { err_msg: String },

    #[error("provided BTE public key does not verify correctly")]
    InvalidBTEPublicKey,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct RegistrationState {
    pub(crate) assigned_index: Option<NodeIndex>,
}

impl RegistrationState {
    /// Specifies whether this dealer has already registered in the particular DKG epoch
    pub fn completed(&self) -> bool {
        self.assigned_index.is_some()
    }
}
