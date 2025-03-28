// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymRewarderError;
use crate::rewarder::nyxd_client::NyxdClient;
use crate::rewarder::storage::RewarderStorage;
use crate::rewarder::ticketbook_issuance::types::TicketbookIssuanceResults;
use crate::rewarder::ticketbook_issuance::verifier::TicketbookIssuanceVerifier;
pub(crate) use crate::rewarder::ticketbook_issuance::verifier::VerificationConfig;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::AccountId;
use std::sync::Arc;
use time::Date;
use tracing::{debug, info};

pub mod helpers;
// mod monitor;
pub mod types;
pub mod verifier;

pub struct TicketbookIssuance {
    pub(crate) config: VerificationConfig,
    pub(crate) nyxd_client: NyxdClient,
    pub(crate) rewarder_keypair: Arc<ed25519::KeyPair>,

    pub(crate) storage: RewarderStorage,
    pub(crate) whitelist: Vec<AccountId>,
}

impl TicketbookIssuance {
    pub(crate) fn new(
        config: VerificationConfig,
        storage: RewarderStorage,
        nyxd_client: &NyxdClient,
        rewarder_keypair: Arc<ed25519::KeyPair>,
        whitelist: &[AccountId],
    ) -> Self {
        TicketbookIssuance {
            config,
            nyxd_client: nyxd_client.clone(),
            rewarder_keypair,
            storage,
            whitelist: whitelist.to_vec(),
        }
    }

    pub(crate) async fn get_issued_ticketbooks_results(
        &self,
        expiration_date: Date,
    ) -> Result<TicketbookIssuanceResults, NymRewarderError> {
        info!("checking for all issued ticketbooks on {expiration_date}");

        // 1. get all ecash issuers
        let issuers = self.nyxd_client.get_current_ticketbook_issuers().await?;
        debug!("retrieved {} ticketbook issuers", issuers.len());

        // 2. load all banned issuers to skip them completely
        let banned = self.storage.load_banned_ticketbook_issuers().await?;
        debug!("retrieved {} banned ticketbook issuers", banned.len());

        let mut verifier = TicketbookIssuanceVerifier::new(
            self.config,
            &self.rewarder_keypair,
            &self.whitelist,
            banned,
            expiration_date,
        );

        // 3. go around and check the specified issuers
        Ok(verifier.check_issuers(issuers).await)
    }
}
