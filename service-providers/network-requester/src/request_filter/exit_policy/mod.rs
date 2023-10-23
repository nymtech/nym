// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkRequesterError;
use log::trace;
use nym_exit_policy::client::get_exit_policy;
use nym_exit_policy::ExitPolicy;
use nym_socks5_requests::RemoteAddress;
use reqwest::IntoUrl;
use tokio::net::lookup_host;
use url::Url;

pub struct ExitPolicyRequestFilter {
    upstream: Option<Url>,
    policy: ExitPolicy,
}

impl ExitPolicyRequestFilter {
    pub(crate) async fn new_upstream(url: impl IntoUrl) -> Result<Self, NetworkRequesterError> {
        let url = url
            .into_url()
            .map_err(|source| NetworkRequesterError::MalformedExitPolicyUpstreamUrl { source })?;

        Ok(ExitPolicyRequestFilter {
            upstream: Some(url.clone()),
            policy: get_exit_policy(url).await?,
        })
    }

    pub(crate) fn new(policy: ExitPolicy) -> Self {
        ExitPolicyRequestFilter {
            upstream: None,
            policy,
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
