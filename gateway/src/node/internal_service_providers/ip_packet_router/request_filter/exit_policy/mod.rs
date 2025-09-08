// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::service_providers::ip_packet_router::error::IpPacketRouterError;
use nym_exit_policy::client::get_exit_policy;
use nym_exit_policy::ExitPolicy;
use std::net::SocketAddr;
use url::Url;

pub struct ExitPolicyRequestFilter {
    #[allow(unused)]
    upstream: Option<Url>,
    policy: ExitPolicy,
}

impl ExitPolicyRequestFilter {
    pub(crate) async fn new_upstream(url: Url) -> Result<Self, IpPacketRouterError> {
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
