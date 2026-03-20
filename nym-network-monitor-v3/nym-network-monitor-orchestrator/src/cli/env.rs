// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Environment variable names used as fallbacks for CLI arguments.
/// Each constant matches the `env = ...` attribute on the corresponding clap field.
pub mod vars {
    // run orchestrator args
    pub const NYM_NETWORK_MONITOR_ORCHESTRATOR_TOKEN_ARG: &str =
        "NYM_NETWORK_MONITOR_ORCHESTRATOR_TOKEN";
}
