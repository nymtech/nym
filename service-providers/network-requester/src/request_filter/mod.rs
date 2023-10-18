// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::request_filter::allowed_hosts::OutboundRequestFilter;
use crate::request_filter::exit_policy::ExitPolicyRequestFilter;
use log::warn;
use nym_socks5_requests::RemoteAddress;

/// Old request filtering based on the allowed.list files.
pub mod allowed_hosts;
pub mod exit_policy;

pub(crate) enum RequestFilter {
    AllowList(OutboundRequestFilter),
    ExitPolicy(ExitPolicyRequestFilter),
}

impl RequestFilter {
    pub(crate) async fn check_address(&mut self, address: &RemoteAddress) -> bool {
        match self {
            RequestFilter::AllowList(old_filter) => old_filter.check(address).await,
            RequestFilter::ExitPolicy(exit_policy) => match exit_policy.check(address).await {
                Err(err) => {
                    warn!("failed to validate '{address}' against the exit policy: {err}");
                    false
                }
                Ok(res) => res,
            },
        }
    }
}
