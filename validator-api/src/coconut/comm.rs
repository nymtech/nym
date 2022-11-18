// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::Result;
use coconut_interface::VerificationKey;
use credentials::coconut::utils::obtain_aggregate_verification_key_new;
use validator_client::CoconutApiClient;

#[async_trait]
pub trait APICommunicationChannel {
    async fn aggregated_verification_key(&self) -> Result<VerificationKey>;
}

pub struct QueryCommunicationChannel {
    coconut_api_clients: Vec<CoconutApiClient>,
}

impl QueryCommunicationChannel {
    pub fn new(coconut_api_clients: Vec<CoconutApiClient>) -> Self {
        QueryCommunicationChannel {
            coconut_api_clients,
        }
    }
}

#[async_trait]
impl APICommunicationChannel for QueryCommunicationChannel {
    async fn aggregated_verification_key(&self) -> Result<VerificationKey> {
        Ok(obtain_aggregate_verification_key_new(&self.coconut_api_clients).await?)
    }
}
