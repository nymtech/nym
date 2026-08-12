// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod models;

/// Route constants and absolute-path helpers for the geolocator HTTP API.
pub mod routes {
    pub const ROOT: &str = "/";
    pub const SWAGGER: &str = "/swagger";
    pub const API: &str = "/api";

    pub mod api {
        pub const V1: &str = "/v1";

        pub mod v1 {
            pub const GEOLOCATION: &str = "/geolocation";

            pub mod geolocation {
                pub const REQUEST_CHECK: &str = "/request-check";
                pub const RECHECK_NODE: &str = "/recheck-node";
                pub const RELAY_SELF_DECLARATION: &str = "/relay-self-declaration";
            }
        }
    }
}
