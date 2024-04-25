// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NetworkRequesterError;
use log::warn;
use nym_socks5_requests::RemoteAddress;
use std::sync::Arc;

pub mod exit_policy;

pub use exit_policy::ExitPolicyRequestFilter;

#[derive(Clone)]
pub struct RequestFilter {
    inner: Arc<ExitPolicyRequestFilter>,
}

impl RequestFilter {
    pub(crate) async fn new(config: &Config) -> Result<Self, NetworkRequesterError> {
        Ok(RequestFilter {
            inner: Arc::new(ExitPolicyRequestFilter::new(config).await?),
        })
    }

    pub fn current_exit_policy_filter(&self) -> &ExitPolicyRequestFilter {
        &self.inner
    }

    pub(crate) async fn check_address(&self, address: &RemoteAddress) -> bool {
        self.inner.check(address).await.unwrap_or_else(|err| {
            warn!("failed to validate '{address}' against the exit policy: {err}");
            false
        })
    }
}
