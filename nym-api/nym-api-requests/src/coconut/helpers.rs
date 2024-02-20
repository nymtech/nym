// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_coconut_interface::BlindedSignature;
use tendermint::hash::Hash;

// recomputes plaintext on the credential nym-api has used for signing
//
// note: this method doesn't have to be reversible so just naively concatenate everything
pub fn issued_credential_plaintext(
    epoch_id: u32,
    tx_hash: Hash,
    blinded_partial_credential: &BlindedSignature,
    bs58_encoded_private_attributes_commitments: &[String],
    public_attributes: &[String],
) -> Vec<u8> {
    epoch_id
        .to_be_bytes()
        .into_iter()
        .chain(tx_hash.as_bytes().iter().copied())
        .chain(blinded_partial_credential.to_bytes())
        .chain(
            bs58_encoded_private_attributes_commitments
                .iter()
                .flat_map(|attr| attr.as_bytes().iter().copied()),
        )
        .chain(
            public_attributes
                .iter()
                .flat_map(|attr| attr.as_bytes().iter().copied()),
        )
        .collect()
}
