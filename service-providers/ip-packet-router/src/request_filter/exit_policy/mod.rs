// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use std::net::SocketAddr;

use crate::error::IpPacketRouterError;
use log::warn;
use nym_bin_common::ip_check::is_global_ip;
use nym_exit_policy::ExitPolicy;
use nym_exit_policy::client::get_exit_policy;
use reqwest::IntoUrl;
use url::Url;

pub struct ExitPolicyRequestFilter {
    #[allow(unused)]
    upstream: Option<Url>,
    policy: ExitPolicy,
    allow_local_ips: bool,
}

impl ExitPolicyRequestFilter {
    pub(crate) async fn new_upstream(
        url: impl IntoUrl,
        allow_local_ips: bool,
    ) -> Result<Self, IpPacketRouterError> {
        let url = url
            .into_url()
            .map_err(|source| IpPacketRouterError::MalformedExitPolicyUpstreamUrl { source })?;

        Ok(ExitPolicyRequestFilter {
            upstream: Some(url.clone()),
            policy: get_exit_policy(url).await?,
            allow_local_ips,
        })
    }

    #[allow(unused)]
    pub(crate) fn new(policy: ExitPolicy, allow_local_ips: bool) -> Self {
        ExitPolicyRequestFilter {
            upstream: None,
            policy,
            allow_local_ips,
        }
    }

    pub fn new_from_policy(policy: ExitPolicy, allow_local_ips: bool) -> Self {
        ExitPolicyRequestFilter {
            upstream: None,
            policy,
            allow_local_ips,
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
        // private ranges are disallowed regardless of policy: end user has
        // no business with internal/private IP ranges of IPR
        if !self.allow_local_ips && !is_global_ip(&addr.ip()) {
            warn!("Rejecting non-global address {addr}");
            return Ok(false);
        }

        self.policy
            .allows_sockaddr(addr)
            .ok_or(IpPacketRouterError::AddressNotCoveredByExitPolicy { addr: *addr })
    }
}
