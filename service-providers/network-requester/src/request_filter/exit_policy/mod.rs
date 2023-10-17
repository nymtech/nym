// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::NetworkRequesterError;
use nym_exit_policy::ExitPolicy;
use nym_socks5_requests::RemoteAddress;
use tokio::net::lookup_host;

pub(crate) struct ExitPolicyRequestFilter {
    policy: ExitPolicy,
}

impl ExitPolicyRequestFilter {
    pub(crate) async fn check(
        &self,
        remote: &RemoteAddress,
    ) -> Result<bool, NetworkRequesterError> {
        // try to convert remote to a proper socket address
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
