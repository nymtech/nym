// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub mod vars {
    pub const NYMNODE_NO_BANNER_ARG: &str = "NYMNODE_NO_BANNER";
    pub const NYMNODE_CONFIG_ENV_FILE_ARG: &str = "NYMNODE_CONFIG_ENV_FILE_ARG";
    pub const NYMNODE_ID_ARG: &str = "NYMNODE_ID";
    pub const NYMNODE_OUTPUT_ARG: &str = "NYMNODE_OUTPUT";
    pub const NYMNODE_CONFIG_PATH_ARG: &str = "NYMNODE_CONFIG";

    pub const NYMNODE_DENY_INIT_ARG: &str = "NYMNODE_DENY_INIT";
    pub const NYMNODE_LOCAL_ARG: &str = "NYMNODE_LOCAL";
    pub const NYMNODE_INIT_ONLY_ARG: &str = "NYMNODE_INIT_ONLY";

    pub const NYMMONDE_WRITE_CONFIG_CHANGES_ARG: &str = "NYMNODE_WRITE_CONFIG_CHANGES";

    pub const NYMNODE_BONDING_INFORMATION_OUTPUT_ARG: &str = "NYMNODE_BONDING_INFORMATION_OUTPUT";

    pub const NYMNODE_MODE_ARG: &str = "NYMNODE_MODE";
    pub const NYMNODE_MODES_ARG: &str = "NYMNODE_MODES";

    pub const NYMNODE_ACCEPT_OPERATOR_TERMS: &str = "NYMNODE_ACCEPT_OPERATOR_TERMS";

    // host:
    pub const NYMNODE_PUBLIC_IPS_ARG: &str = "NYMNODE_PUBLIC_IPS";
    pub const NYMNODE_HOSTNAME_ARG: &str = "NYMNODE_HOSTNAME";
    pub const NYMNODE_LOCATION_ARG: &str = "NYMNODE_LOCATION";

    // http:
    pub const NYMNODE_HTTP_BIND_ADDRESS_ARG: &str = "NYMNODE_HTTP_BIND_ADDRESS";
    pub const NYMNODE_HTTP_LANDING_ASSETS_ARG: &str = "NYMNODE_HTTP_LANDING_ASSETS";
    pub const NYMNODE_HTTP_ACCESS_TOKEN_ARG: &str = "NYMNODE_HTTP_ACCESS_TOKEN";
    pub const NYMNODE_HTTP_EXPOSE_SYSTEM_INFO_ARG: &str = "NYMNODE_HTTP_EXPOSE_SYSTEM_INFO";
    pub const NYMNODE_HTTP_EXPOSE_SYSTEM_HARDWARE_ARG: &str = "NYMNODE_HTTP_EXPOSE_SYSTEM_HARDWARE";
    pub const NYMNODE_HTTP_EXPOSE_CRYPTO_HARDWARE_ARG: &str = "NYMNODE_HTTP_EXPOSE_CRYPTO_HARDWARE";

    // mixnet:
    pub const NYMNODE_MIXNET_BIND_ADDRESS_ARG: &str = "NYMNODE_MIXNET_BIND_ADDRESS";
    pub const NYMNODE_MIXNET_ANNOUNCE_PORT_ARG: &str = "NYMNODE_MIXNET_ANNOUNCE_PORT";
    pub const NYMNODE_NYM_APIS_ARG: &str = "NYMNODE_NYM_APIS";
    pub const NYMNODE_NYXD_URLS_ARG: &str = "NYMNODE_NYXD";
    pub const NYMNODE_UNSAFE_DISABLE_NOISE: &str = "UNSAFE_DISABLE_NOISE";
    pub const NYMNODE_UNSAFE_DISABLE_REPLAY_PROTECTION: &str = "UNSAFE_DISABLE_REPLAY_PROTECTION";

    // wireguard:
    pub const NYMNODE_WG_ENABLED_ARG: &str = "NYMNODE_WG_ENABLED";
    pub const NYMNODE_WG_BIND_ADDRESS_ARG: &str = "NYMNODE_WG_BIND_ADDRESS";
    pub const NYMNODE_WG_ANNOUNCED_PORT_ARG: &str = "NYMNODE_WG_ANNOUNCED_PORT";
    pub const NYMNODE_WG_PRIVATE_NETWORK_PREFIX_ARG: &str = "NYMNODE_WG_PRIVATE_NETWORK_PREFIX";

    // verloc:
    pub const NYMNODE_VERLOC_BIND_ADDRESS_ARG: &str = "NYMNODE_VERLOC_BIND_ADDRESS";
    pub const NYMNODE_VERLOC_ANNOUNCE_PORT_ARG: &str = "NYMNODE_VERLOC_ANNOUNCE_PORT";

    // metrics
    pub const NYMNODE_ENABLE_CONSOLE_LOGGING: &str = "NYMNODE_ENABLE_CONSOLE_LOGGING";

    // entry gateway:
    pub const NYMNODE_ENTRY_BIND_ADDRESS_ARG: &str = "NYMNODE_ENTRY_BIND_ADDRESS";
    pub const NYMNODE_ENTRY_ANNOUNCE_WS_PORT_ARG: &str = "NYMNODE_ENTRY_ANNOUNCE_WS_PORT";
    pub const NYMNODE_ENTRY_ANNOUNCE_WSS_PORT_ARG: &str = "NYMNODE_ENTRY_ANNOUNCE_WSS_PORT";
    pub const NYMNODE_ENFORCE_ZK_NYMS_ARG: &str = "NYMNODE_ENFORCE_ZK_NYMS";
    pub const NYMNODE_MNEMONIC_ARG: &str = "NYMNODE_MNEMONIC";

    // exit gateway:
    pub const NYMNODE_UPSTREAM_EXIT_POLICY_ARG: &str = "NYMNODE_UPSTREAM_EXIT_POLICY";
    pub const NYMNODE_OPEN_PROXY_ARG: &str = "NYMNODE_OPEN_PROXY";
}
