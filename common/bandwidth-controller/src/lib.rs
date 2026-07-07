// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_credentials::ecash::bandwidth::CredentialSpendingData;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::OffsetDateTime;
use nym_validator_client::nym_api::EpochId;

pub use controller::BandwidthController;
pub use fetcher::NyxdGlobalDataFetcher;
pub use traits::{BandwidthTicketProvider, FetcherError};

pub mod acquire;
mod controller;
pub mod error;
mod fetcher;
pub mod mock;
pub mod requests;
mod traits;
mod utils;

pub const DEFAULT_TICKETS_TO_SPEND: u32 = 1;

pub const UPGRADE_MODE_JWT_TYPE: &str = "UPGRADE_MODE_JWT";

#[derive(Clone, Debug)]
pub struct PreparedCredential {
    /// The cryptographic material required for spending the underlying credential.
    pub data: CredentialSpendingData,

    /// The (DKG) epoch id under which the credential has been issued so that the verifier
    /// could use correct verification key for validation.
    pub epoch_id: EpochId,

    /// Auxiliary metadata associated with the withdrawn credential
    pub metadata: PreparedCredentialMetadata,
}

#[derive(Copy, Clone, Debug)]
pub struct PreparedCredentialMetadata {
    /// The database id of the stored credential.
    pub ticketbook_id: i64,

    /// The number of tickets withdrawn in this credential
    pub tickets_withdrawn: u32,

    /// The amount of tickets used INCLUDING those tickets that JUST got withdrawn
    pub used_tickets: u32,
}

#[derive(Copy, Clone, Debug)]
pub struct EcashTicketRequest {
    pub ticket_type: TicketType,
    pub gateway_id: ed25519::PublicKey,
    pub tickets_to_spend: u32,
    pub spend_time: OffsetDateTime,
}
