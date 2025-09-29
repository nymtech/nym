// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use nym_api_requests::ecash::BlindSignRequestBody;
use nym_cache::CachedImmutableItems;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::scheme::coin_indices_signatures::AnnotatedCoinIndexSignature;
use nym_compact_ecash::scheme::expiration_date_signatures::AnnotatedExpirationDateSignature;
use nym_compact_ecash::scheme::keygen::SecretKeyAuth;
use nym_compact_ecash::{BlindedSignature, EncodedDate, EncodedTicketType};
use nym_compact_ecash::{PublicKeyUser, WithdrawalRequest};
use nym_ecash_time::EcashTime;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct IssuedExpirationDateSignatures {
    pub(crate) epoch_id: EpochId,
    pub(crate) signatures: Vec<AnnotatedExpirationDateSignature>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct IssuedCoinIndicesSignatures {
    pub(crate) epoch_id: EpochId,
    pub(crate) signatures: Vec<AnnotatedCoinIndexSignature>,
}

pub(crate) trait CredentialRequest {
    fn withdrawal_request(&self) -> &WithdrawalRequest;
    fn expiration_date_timestamp(&self) -> EncodedDate;
    fn ticketbook_type(&self) -> EncodedTicketType;
    fn ecash_pubkey(&self) -> PublicKeyUser;
}

impl CredentialRequest for BlindSignRequestBody {
    fn withdrawal_request(&self) -> &WithdrawalRequest {
        &self.inner_sign_request
    }

    fn expiration_date_timestamp(&self) -> EncodedDate {
        self.expiration_date.ecash_unix_timestamp()
    }

    fn ticketbook_type(&self) -> EncodedTicketType {
        self.ticketbook_type.encode()
    }

    fn ecash_pubkey(&self) -> PublicKeyUser {
        self.ecash_pubkey
    }
}

pub(crate) fn blind_sign<C: CredentialRequest>(
    request: &C,
    signing_key: &SecretKeyAuth,
) -> Result<BlindedSignature, EcashError> {
    Ok(nym_compact_ecash::scheme::withdrawal::issue(
        signing_key,
        request.ecash_pubkey(),
        request.withdrawal_request(),
        request.expiration_date_timestamp(),
        request.ticketbook_type(),
    )?)
}

// an item that stays constant throughout given epoch
pub(crate) type CachedImmutableEpochItem<T> = CachedImmutableItems<EpochId, T>;
