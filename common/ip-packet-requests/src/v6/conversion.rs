// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{v6, v7};

impl From<v7::response::StaticConnectFailureReason> for v6::response::StaticConnectFailureReason {
    fn from(failure: v7::response::StaticConnectFailureReason) -> Self {
        match failure {
            v7::response::StaticConnectFailureReason::RequestedIpAlreadyInUse => {
                v6::response::StaticConnectFailureReason::RequestedIpAlreadyInUse
            }
            v7::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse => {
                v6::response::StaticConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            v7::response::StaticConnectFailureReason::OutOfDateTimestamp => {
                v6::response::StaticConnectFailureReason::Other("out of date timestamp".to_string())
            }
            v7::response::StaticConnectFailureReason::Other(reason) => {
                v6::response::StaticConnectFailureReason::Other(reason)
            }
        }
    }
}

impl From<v7::response::DynamicConnectFailureReason> for v6::response::DynamicConnectFailureReason {
    fn from(failure: v7::response::DynamicConnectFailureReason) -> Self {
        match failure {
            v7::response::DynamicConnectFailureReason::RequestedNymAddressAlreadyInUse => {
                v6::response::DynamicConnectFailureReason::RequestedNymAddressAlreadyInUse
            }
            v7::response::DynamicConnectFailureReason::NoAvailableIp => {
                v6::response::DynamicConnectFailureReason::NoAvailableIp
            }
            v7::response::DynamicConnectFailureReason::Other(err) => {
                v6::response::DynamicConnectFailureReason::Other(err)
            }
        }
    }
}

impl From<v7::response::InfoResponseReply> for v6::response::InfoResponseReply {
    fn from(reply: v7::response::InfoResponseReply) -> Self {
        match reply {
            v7::response::InfoResponseReply::Generic { msg } => {
                v6::response::InfoResponseReply::Generic { msg }
            }
            v7::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            } => v6::response::InfoResponseReply::VersionMismatch {
                request_version,
                response_version,
            },
            v7::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst } => {
                v6::response::InfoResponseReply::ExitPolicyFilterCheckFailed { dst }
            }
        }
    }
}

impl From<v7::response::InfoLevel> for v6::response::InfoLevel {
    fn from(level: v7::response::InfoLevel) -> Self {
        match level {
            v7::response::InfoLevel::Info => v6::response::InfoLevel::Info,
            v7::response::InfoLevel::Warn => v6::response::InfoLevel::Warn,
            v7::response::InfoLevel::Error => v6::response::InfoLevel::Error,
        }
    }
}
