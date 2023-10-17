// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::request_filter::allowed_hosts::OutboundRequestFilter;
use crate::request_filter::exit_policy::ExitPolicyRequestFilter;
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
            RequestFilter::AllowList(old_filter) => {
                old_filter.check(address).await;
                todo!()
            }
            RequestFilter::ExitPolicy(exit_policy) => {
                exit_policy.check(address).await;
                todo!()
            }
        }
    }
}
