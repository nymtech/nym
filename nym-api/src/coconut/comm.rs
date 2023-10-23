// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::error::Result;
use crate::nyxd;
use nym_coconut_dkg_common::types::EpochId;
use nym_compact_ecash::VerificationKeyAuth;
use nym_credentials::coconut::utils::obtain_aggregate_verification_key;
use nym_validator_client::coconut::all_coconut_api_clients;
use std::ops::Deref;

#[async_trait]
pub trait APICommunicationChannel {
    async fn aggregated_verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth>;
}

pub(crate) struct QueryCommunicationChannel {
    nyxd_client: nyxd::Client,
}

impl QueryCommunicationChannel {
    pub fn new(nyxd_client: nyxd::Client) -> Self {
        QueryCommunicationChannel { nyxd_client }
    }
}

#[async_trait]
impl APICommunicationChannel for QueryCommunicationChannel {
    async fn aggregated_verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth> {
        let client = self.nyxd_client.0.read().await;
        let coconut_api_clients = all_coconut_api_clients(client.deref(), epoch_id).await?;
        let vk = obtain_aggregate_verification_key(&coconut_api_clients).await?;
        Ok(vk)
    }
}
