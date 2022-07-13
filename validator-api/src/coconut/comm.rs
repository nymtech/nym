// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::Result;
use coconut_interface::VerificationKey;
use credentials::obtain_aggregate_verification_key;
use url::Url;

#[async_trait]
pub trait APICommunicationChannel {
    async fn aggregated_verification_key(&self) -> Result<VerificationKey>;
}

pub struct QueryCommunicationChannel {
    validator_apis: Vec<Url>,
}

impl QueryCommunicationChannel {
    pub fn new(validator_apis: Vec<Url>) -> Self {
        QueryCommunicationChannel { validator_apis }
    }
}

#[async_trait]
impl APICommunicationChannel for QueryCommunicationChannel {
    async fn aggregated_verification_key(&self) -> Result<VerificationKey> {
        Ok(obtain_aggregate_verification_key(&self.validator_apis).await?)
    }
}
