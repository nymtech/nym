// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_compact_ecash::BlindedSignature;
use nym_credentials_interface::TicketType;
use nym_ecash_time::EcashTime;
use std::iter::once;
use time::Date;

// recomputes plaintext on the credential nym-api has used for signing
//
// note: this method doesn't have to be reversible so just naively concatenate everything
pub fn issued_credential_plaintext(
    epoch_id: u32,
    deposit_id: u32,
    blinded_partial_credential: &BlindedSignature,
    encoded_private_attributes_commitments: &[Vec<u8>],
    expiration_date: Date,
    ticketbook_type: TicketType,
) -> Vec<u8> {
    epoch_id
        .to_be_bytes()
        .into_iter()
        .chain(deposit_id.to_be_bytes())
        .chain(blinded_partial_credential.to_bytes())
        .chain(
            encoded_private_attributes_commitments
                .iter()
                .flat_map(|attr| attr.iter().copied()),
        )
        .chain(expiration_date.ecash_unix_timestamp().to_be_bytes())
        .chain(once(ticketbook_type.to_repr() as u8))
        .collect()
}
