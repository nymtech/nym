// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NetworkRequesterError;
use log::{trace, warn};
use nym_bin_common::ip_check::is_global_ip;
use nym_exit_policy::ExitPolicy;
use nym_exit_policy::client::get_exit_policy;
use nym_socks5_requests::RemoteAddress;
use reqwest::IntoUrl;
use tokio::net::lookup_host;
use url::Url;

pub struct ExitPolicyRequestFilter {
    upstream: Option<Url>,
    policy: ExitPolicy,
    allow_local_ips: bool,
}

impl From<ExitPolicy> for ExitPolicyRequestFilter {
    fn from(value: ExitPolicy) -> Self {
        ExitPolicyRequestFilter::new_from_policy(value, false)
    }
}

impl ExitPolicyRequestFilter {
    pub(crate) async fn new_upstream(
        url: impl IntoUrl,
        allow_local_ips: bool,
    ) -> Result<Self, NetworkRequesterError> {
        let url = url
            .into_url()
            .map_err(|source| NetworkRequesterError::MalformedExitPolicyUpstreamUrl { source })?;

        Ok(ExitPolicyRequestFilter {
            upstream: Some(url.clone()),
            policy: get_exit_policy(url).await?,
            allow_local_ips,
        })
    }

    pub(crate) async fn new(config: &Config) -> Result<Self, NetworkRequesterError> {
        let allow_local_ips = config.network_requester.allow_local_ips;
        if allow_local_ips {
            warn!(
                "Requests to non-global destinations are allowed by the policy guard. \
                 This is intended for local development and NOT recommended in production \
                 unless you know what you're doing."
            );
        }
        let policy_filter = if config.network_requester.open_proxy {
            ExitPolicyRequestFilter::new_from_policy(ExitPolicy::new_open(), allow_local_ips)
        } else {
            let upstream_url = config
                .network_requester
                .upstream_exit_policy_url
                .as_ref()
                .ok_or(NetworkRequesterError::NoUpstreamExitPolicy)?;
            ExitPolicyRequestFilter::new_upstream(upstream_url.clone(), allow_local_ips).await?
        };
        Ok(policy_filter)
    }

    pub fn new_from_policy(policy: ExitPolicy, allow_local_ips: bool) -> Self {
        ExitPolicyRequestFilter {
            upstream: None,
            policy,
            allow_local_ips,
        }
    }

    pub fn policy(&self) -> &ExitPolicy {
        &self.policy
    }

    pub fn upstream(&self) -> Option<&Url> {
        self.upstream.as_ref()
    }

    pub(crate) async fn check(
        &self,
        remote: &RemoteAddress,
    ) -> Result<bool, NetworkRequesterError> {
        // try to convert the remote to a proper socket address
        let addrs = lookup_host(remote)
            .await
            .map_err(|source| NetworkRequesterError::CouldNotResolveHost {
                remote: remote.to_string(),
                source,
            })?
            .collect::<Vec<_>>();

        // I'm honestly not sure if it's possible to return an Ok with an empty iterator,
        // but might as well guard against that
        if addrs.is_empty() {
            return Err(NetworkRequesterError::EmptyResolvedAddresses {
                remote: remote.to_string(),
            });
        }

        trace!("{remote} has been resolved to {addrs:?}");

        // if the remote decided to give us an address that can resolve to multiple socket addresses,
        // they'd better make sure all of them are allowed by the exit policy.
        for addr in addrs {
            // private ranges are disallowed regardless of policy: end user has
            // no business with internal/private IP ranges of network requester
            if !self.allow_local_ips && !is_global_ip(&addr.ip()) {
                warn!("Rejecting non-global address {addr} for '{remote}'");
                return Ok(false);
            }
            // exit policy determines which PUBLIC facing addresses are allowed
            if !self
                .policy
                .allows_sockaddr(&addr)
                .ok_or(NetworkRequesterError::AddressNotCoveredByExitPolicy { addr })?
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}
