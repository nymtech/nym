// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// What can go wrong between a provider and the gateway hosting it.
///
/// Short, because most of what could go wrong on a client's LP path cannot go wrong here: there is
/// no socket to fail, no session to be missing, and no peer to be unreachable - the far end is the
/// same process.
#[derive(Debug, Error)]
pub enum LpProviderError {
    #[error("the sphinx packet could not be peeled: {0}")]
    Peel(String),

    #[error("the gateway is no longer taking frames from this provider")]
    GatewayGone,

    #[error("could not read the {provider} keys its LP pipelines peel with: {source}")]
    UnreadableKeys {
        provider: &'static str,
        source: std::io::Error,
    },
}
