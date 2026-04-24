// Copyright 2026 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub mod api;
pub mod models;

#[cfg(feature = "client")]
pub mod client;

/// Generates a function that returns the full absolute path for a route
/// by concatenating a parent prefix with a suffix.
macro_rules! absolute_route {
    ( $name:ident, $parent:expr, $suffix:expr ) => {
        pub fn $name() -> String {
            format!("{}{}", $parent, $suffix)
        }
    };
}

/// Route constants and absolute-path helpers for the orchestrator HTTP API.
/// Used by both the orchestrator server (for route registration) and the agent
/// client (for constructing request URLs).
pub mod routes {
    pub const ROOT: &str = "/";
    pub const V1: &str = "/v1";
    pub const SWAGGER: &str = "/swagger";

    pub mod v1 {
        pub const AGENT: &str = "/agent";
        pub const METRICS: &str = "/metrics";
        pub const RESULTS: &str = "/results";

        absolute_route!(agent_absolute, super::V1, AGENT);
        absolute_route!(metrics_absolute, super::V1, METRICS);
        absolute_route!(results_absolute, super::V1, RESULTS);

        pub mod agent {
            use super::*;

            pub const PORT_REQUEST: &str = "/port-request";
            pub const ANNOUNCE: &str = "/announce";
            pub const REQUEST_TESTRUN: &str = "/request-testrun";
            pub const SUBMIT_TESTRUN_RESULT: &str = "/submit-testrun-result";

            absolute_route!(port_request_absolute, agent_absolute(), PORT_REQUEST);
            absolute_route!(announce_absolute, agent_absolute(), ANNOUNCE);
            absolute_route!(request_testrun_absolute, agent_absolute(), REQUEST_TESTRUN);
            absolute_route!(
                submit_testrun_absolute,
                agent_absolute(),
                SUBMIT_TESTRUN_RESULT
            );
        }

        pub mod metrics {
            // use super::*;
        }

        pub mod results {
            // use super::*;
        }
    }
}
