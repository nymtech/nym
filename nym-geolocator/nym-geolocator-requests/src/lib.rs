// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "client")]
pub mod client;
pub mod models;

/// Generates a function that returns the full absolute path for a route
/// by concatenating a parent prefix with a suffix.
macro_rules! absolute_route {
    ( $name:ident, $parent:expr, $suffix:expr ) => {
        pub fn $name() -> String {
            format!("{}{}", $parent, $suffix)
        }
    };
}

/// Route constants and absolute-path helpers for the geolocator HTTP API.
pub mod routes {
    pub const ROOT: &str = "/";
    pub const SWAGGER: &str = "/swagger";
    pub const API: &str = "/api";

    pub mod api {
        pub const V1: &str = "/v1";

        absolute_route!(v1_absolute, super::API, V1);

        pub mod v1 {
            pub const GEOLOCATION: &str = "/geolocation";

            absolute_route!(geolocation_absolute, super::v1_absolute(), GEOLOCATION);

            pub mod geolocation {
                pub const REQUEST_CHECK: &str = "/request-check";
                pub const RECHECK_NODE: &str = "/recheck-node";
                pub const RELAY_SELF_DECLARATION: &str = "/relay-self-declaration";

                absolute_route!(
                    request_check_absolute,
                    super::geolocation_absolute(),
                    REQUEST_CHECK
                );
                absolute_route!(
                    recheck_node_absolute,
                    super::geolocation_absolute(),
                    RECHECK_NODE
                );
                absolute_route!(
                    relay_self_declaration_absolute,
                    super::geolocation_absolute(),
                    RELAY_SELF_DECLARATION
                );
            }
        }
    }
}
