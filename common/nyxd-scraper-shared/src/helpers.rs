// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::block_processor::types::ParsedTransactionResponse;
use crate::constants::{BECH32_CONESNSUS_PUBKEY_PREFIX, BECH32_CONSENSUS_ADDRESS_PREFIX};
use cosmrs::AccountId;
use sha2::{Digest, Sha256};
use tendermint::{account, PublicKey};
use tendermint::{validator, Hash};
use tendermint_rpc::endpoint::validators;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MalformedDataError {
    #[error("failed to parse validator's address: {source}")]
    MalformedValidatorAddress {
        #[source]
        source: eyre::Report,
    },

    #[error("failed to parse validator's address: {source}")]
    MalformedValidatorPubkey {
        #[source]
        source: eyre::Report,
    },

    #[error(
        "could not find validator information for {address}; the validator has signed a commit"
    )]
    MissingValidatorInfoCommitted { address: String },
}

pub fn tx_hash<M: AsRef<[u8]>>(raw_tx: M) -> Hash {
    Hash::Sha256(Sha256::digest(raw_tx).into())
}

pub fn validator_pubkey_to_bech32(pubkey: PublicKey) -> Result<AccountId, MalformedDataError> {
    // TODO: this one seem to attach additional prefix to they pubkeys, is that what we want instead maybe?
    // Ok(pubkey.to_bech32(BECH32_CONESNSUS_PUBKEY_PREFIX))
    AccountId::new(BECH32_CONESNSUS_PUBKEY_PREFIX, &pubkey.to_bytes())
        .map_err(|source| MalformedDataError::MalformedValidatorPubkey { source })
}

pub fn validator_consensus_address(id: account::Id) -> Result<AccountId, MalformedDataError> {
    AccountId::new(BECH32_CONSENSUS_ADDRESS_PREFIX, id.as_ref())
        .map_err(|source| MalformedDataError::MalformedValidatorAddress { source })
}

pub fn tx_gas_sum(txs: &[ParsedTransactionResponse]) -> i64 {
    txs.iter().map(|tx| tx.tx_result.gas_used).sum()
}

pub fn validator_info(
    id: account::Id,
    validators: &validators::Response,
) -> Result<&validator::Info, MalformedDataError> {
    match validators.validators.iter().find(|v| v.address == id) {
        Some(info) => Ok(info),
        None => {
            let addr = validator_consensus_address(id)?;
            Err(MalformedDataError::MissingValidatorInfoCommitted {
                address: addr.to_string(),
            })
        }
    }
}
