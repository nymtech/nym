// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

pub mod api;
pub mod error;

macro_rules! absolute_route {
    ( $name:ident, $parent:expr, $suffix:expr ) => {
        pub fn $name() -> String {
            format!("{}{}", $parent, $suffix)
        }
    };
}

// still thinking how to nicely organise it
pub mod routes {
    pub const LANDING_PAGE: &str = "/";
    pub const ROOT: &str = "/";
    pub const API: &str = "/api";

    pub mod api {
        pub const V1: &str = "/v1";

        absolute_route!(v1_absolute, super::API, V1);

        pub mod v1 {
            use super::*;

            pub const ROLES: &str = "/roles";
            pub const BUILD_INFO: &str = "/build-information";
            pub const HOST_INFO: &str = "/host-information";
            pub const SYSTEM_INFO: &str = "/system-info";
            pub const NODE_DESCRIPTION: &str = "/description";
            pub const HEALTH: &str = "/health";
            pub const SWAGGER: &str = "/swagger";

            pub const GATEWAY: &str = "/gateway";
            pub const MIXNODE: &str = "/mixnode";
            pub const METRICS: &str = "/metrics";
            pub const NETWORK_REQUESTER: &str = "/network-requester";
            pub const IP_PACKET_ROUTER: &str = "/ip-packet-router";

            // define helper functions to get absolute routes
            absolute_route!(health_absolute, v1_absolute(), HEALTH);
            absolute_route!(roles_absolute, v1_absolute(), ROLES);
            absolute_route!(build_info_absolute, v1_absolute(), BUILD_INFO);
            absolute_route!(host_info_absolute, v1_absolute(), HOST_INFO);
            absolute_route!(system_info_absolute, v1_absolute(), SYSTEM_INFO);
            absolute_route!(description_absolute, v1_absolute(), NODE_DESCRIPTION);

            absolute_route!(gateway_absolute, v1_absolute(), GATEWAY);
            absolute_route!(mixnode_absolute, v1_absolute(), MIXNODE);
            absolute_route!(metrics_absolute, v1_absolute(), METRICS);
            absolute_route!(network_requester_absolute, v1_absolute(), NETWORK_REQUESTER);
            absolute_route!(ip_packet_router_absolute, v1_absolute(), IP_PACKET_ROUTER);
            absolute_route!(swagger_absolute, v1_absolute(), SWAGGER);

            pub mod metrics {
                use super::*;

                pub const MIXING: &str = "/mixing";

                absolute_route!(mixing_absolute, metrics_absolute(), MIXING);
            }

            pub mod gateway {
                use super::*;

                pub const CLIENT_INTERFACES: &str = "/client-interfaces";

                absolute_route!(
                    client_interfaces_absolute,
                    gateway_absolute(),
                    CLIENT_INTERFACES
                );

                pub mod client_interfaces {
                    use super::*;

                    pub const WIREGUARD: &str = "/wireguard";
                    pub const WEBSOCKETS: &str = "/mixnet-websockets";

                    absolute_route!(wireguard_absolute, client_interfaces_absolute(), WIREGUARD);
                    absolute_route!(
                        mixnet_websockets_absolute,
                        client_interfaces_absolute(),
                        WEBSOCKETS
                    );

                    pub mod wireguard {
                        use super::*;

                        pub const CLIENT: &str = "/client";
                        pub const CLIENTS: &str = "/clients";

                        absolute_route!(client_absolute, wireguard_absolute(), CLIENT);
                        absolute_route!(clients_absolute, wireguard_absolute(), CLIENTS);
                    }
                }
            }

            pub mod mixnode {
                // use super::*;
            }

            pub mod network_requester {
                use super::*;

                pub const EXIT_POLICY: &str = "/exit-policy";

                absolute_route!(
                    exit_policy_absolute,
                    network_requester_absolute(),
                    EXIT_POLICY
                );
            }

            pub mod ip_packet_router {
                // use super::*;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_expansion_works() {
        assert_eq!("/api/v1", routes::api::v1_absolute());

        assert_eq!(
            "/api/v1/build-information",
            routes::api::v1::build_info_absolute()
        );
        assert_eq!(
            "/api/v1/host-information",
            routes::api::v1::host_info_absolute()
        );
        assert_eq!("/api/v1/roles", routes::api::v1::roles_absolute());

        assert_eq!("/api/v1/gateway", routes::api::v1::gateway_absolute());
        assert_eq!(
            "/api/v1/gateway/client-interfaces",
            routes::api::v1::gateway::client_interfaces_absolute()
        );
        assert_eq!(
            "/api/v1/gateway/client-interfaces/wireguard",
            routes::api::v1::gateway::client_interfaces::wireguard_absolute()
        );
        assert_eq!(
            "/api/v1/gateway/client-interfaces/mixnet-websockets",
            routes::api::v1::gateway::client_interfaces::mixnet_websockets_absolute()
        );

        assert_eq!("/api/v1/mixnode", routes::api::v1::mixnode_absolute());
        assert_eq!(
            "/api/v1/network-requester",
            routes::api::v1::network_requester_absolute()
        );
        assert_eq!(
            "/api/v1/ip-packet-router",
            routes::api::v1::ip_packet_router_absolute()
        );
    }
}
