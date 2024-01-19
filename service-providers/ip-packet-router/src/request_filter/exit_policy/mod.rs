// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::SocketAddr;

use crate::error::IpPacketRouterError;
use nym_exit_policy::client::get_exit_policy;
use nym_exit_policy::ExitPolicy;
use reqwest::IntoUrl;
use url::Url;

pub struct ExitPolicyRequestFilter {
    #[allow(unused)]
    upstream: Option<Url>,
    policy: ExitPolicy,
}

impl ExitPolicyRequestFilter {
    pub(crate) async fn new_upstream(url: impl IntoUrl) -> Result<Self, IpPacketRouterError> {
        let url = url
            .into_url()
            .map_err(|source| IpPacketRouterError::MalformedExitPolicyUpstreamUrl { source })?;

        Ok(ExitPolicyRequestFilter {
            upstream: Some(url.clone()),
            policy: get_exit_policy(url).await?,
        })
    }

    #[allow(unused)]
    pub(crate) fn new(policy: ExitPolicy) -> Self {
        ExitPolicyRequestFilter {
            upstream: None,
            policy,
        }
    }

    #[allow(unused)]
    pub fn policy(&self) -> &ExitPolicy {
        &self.policy
    }

    #[allow(unused)]
    pub fn upstream(&self) -> Option<&Url> {
        self.upstream.as_ref()
    }

    pub(crate) async fn check(&self, addr: &SocketAddr) -> Result<bool, IpPacketRouterError> {
        self.policy
            .allows_sockaddr(addr)
            .ok_or(IpPacketRouterError::AddressNotCoveredByExitPolicy { addr: *addr })
    }
}
