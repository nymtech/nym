// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::signers_cache::cache::SignersCacheData;
use crate::support::caching::refresher::CacheItemProvider;
use crate::support::nyxd::Client;
use async_trait::async_trait;
use nym_ecash_signer_check::{check_signers_with_client, SignerCheckError};

pub(crate) struct SignersCacheDataProvider {
    nyxd_client: Client,
}

#[async_trait]
impl CacheItemProvider for SignersCacheDataProvider {
    type Item = SignersCacheData;
    type Error = SignerCheckError;

    async fn try_refresh(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        self.refresh().await.map(Some)
    }
}

impl SignersCacheDataProvider {
    pub(crate) fn new(nyxd_client: Client) -> Self {
        SignersCacheDataProvider { nyxd_client }
    }

    async fn refresh(&self) -> Result<SignersCacheData, SignerCheckError> {
        let signers_results = check_signers_with_client(&self.nyxd_client).await?;
        Ok(SignersCacheData { signers_results })
    }
}
