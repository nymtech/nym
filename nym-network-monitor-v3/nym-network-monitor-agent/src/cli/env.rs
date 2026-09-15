// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Environment variable names used as fallbacks for CLI arguments.
/// Each constant matches the `env = ...` attribute on the corresponding clap field.
pub mod vars {
    // common args
    pub const NYM_NETWORK_MONITOR_AGENT_SENDING_DURATION_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_SENDING_DURATION";
    pub const NYM_NETWORK_MONITOR_AGENT_WAITING_DURATION_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_WAITING_DURATION";
    pub const NYM_NETWORK_MONITOR_AGENT_TARGET_RATE_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_TARGET_RATE";
    pub const NYM_NETWORK_MONITOR_AGENT_REUSE_HEADER_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_REUSE_HEADER";
    pub const NYM_NETWORK_MONITOR_AGENT_NOISE_KEY_PATH_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_NOISE_KEY_PATH";
    pub const NYM_NETWORK_MONITOR_AGENT_PACKET_DELAY_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_PACKET_DELAY";
    pub const NYM_NETWORK_MONITOR_AGENT_EGRESS_CONNECTION_TIMEOUT_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_EGRESS_CONNECTION_TIMEOUT";
    pub const NYM_NETWORK_MONITOR_AGENT_NOISE_HANDSHAKE_TIMEOUT_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_NOISE_HANDSHAKE_TIMEOUT";
    pub const NYM_NETWORK_MONITOR_AGENT_SENDING_BATCH_SIZE_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_SENDING_BATCH_SIZE";
    pub const NYM_NETWORK_MONITOR_AGENT_BIND_ADDRESS_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_BIND_ADDRESS";

    // liveness profile args. every value is provisional, so each one has to be movable in a
    // deployment without a rebuild
    pub const NYM_NETWORK_MONITOR_AGENT_LIVENESS_PACKETS_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_LIVENESS_PACKETS";
    pub const NYM_NETWORK_MONITOR_AGENT_LIVENESS_TARGET_RATE_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_LIVENESS_TARGET_RATE";
    pub const NYM_NETWORK_MONITOR_AGENT_LIVENESS_WAITING_DURATION_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_LIVENESS_WAITING_DURATION";
    pub const NYM_NETWORK_MONITOR_AGENT_LIVENESS_PER_TARGET_TIMEOUT_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_LIVENESS_PER_TARGET_TIMEOUT";

    // gateway client session args. the websocket leg of a gateway probe is deliberately NOT held to
    // the mixnet timeouts: an http upgrade and a registration handshake are not a TCP connect and a
    // Noise handshake, so sharing their knobs would tie two unrelated waits together
    pub const NYM_NETWORK_MONITOR_AGENT_SESSION_CONNECT_TIMEOUT_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_SESSION_CONNECT_TIMEOUT";
    pub const NYM_NETWORK_MONITOR_AGENT_SESSION_REGISTRATION_TIMEOUT_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_SESSION_REGISTRATION_TIMEOUT";

    // run agent args
    pub const NYM_NETWORK_MONITOR_AGENT_ORCHESTRATOR_ADDRESS_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_ORCHESTRATOR_ADDRESS";
    pub const NYM_NETWORK_MONITOR_AGENT_ORCHESTRATOR_TOKEN_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_ORCHESTRATOR_TOKEN";
    pub const NYM_NETWORK_MONITOR_AGENT_HOST_IPV4_ARG: &str = "NYM_NETWORK_MONITOR_AGENT_HOST_IPV4";
    pub const NYM_NETWORK_MONITOR_AGENT_HOST_IPV6_ARG: &str = "NYM_NETWORK_MONITOR_AGENT_HOST_IPV6";
    pub const NYM_NETWORK_MONITOR_AGENT_HOST_PORT_ARG: &str = "NYM_NETWORK_MONITOR_AGENT_HOST_PORT";

    // manual `test-*` args. the node is named by its http api ALONE: its identity, noise and sphinx
    // keys, key rotation, mix port, announced addresses and client websocket port are all read off
    // the node itself, so none of them is an argument
    pub const NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_V4_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_V4";
    pub const NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_V6_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_MIXNET_ADDRESS_V6";
    pub const NYM_NETWORK_MONITOR_AGENT_NODE_HOST_ARG: &str = "NYM_NETWORK_MONITOR_AGENT_NODE_HOST";
    pub const NYM_NETWORK_MONITOR_AGENT_NODE_HTTP_PORT_ARG: &str =
        "NYM_NETWORK_MONITOR_AGENT_NODE_HTTP_PORT";
}
