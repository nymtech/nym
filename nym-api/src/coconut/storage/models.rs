// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub struct EpochCredentials {
    pub epoch_id: u32,
    pub start_id: i64,
    pub total_issued: u32,
}

pub struct IssuedCredential {
    pub id: i64,
    pub epoch_id: u32,
    pub tx_hash: String,
    pub bs58_partial_credential: String,
    pub bs58_signature: String,
    // TODO: missing blindsignrequest
}
