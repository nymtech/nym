// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "network")]
use crate::{ApiUrlConst, DenomDetails, ValidatorDetails};

pub const NETWORK_NAME: &str = "mainnet";

pub const BECH32_PREFIX: &str = "n";

#[cfg(feature = "network")]
pub const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
#[cfg(feature = "network")]
pub const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

pub const MIXNET_CONTRACT_ADDRESS: &str =
    "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr";
pub const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";

// \/ TODO: this has to be updated once the contract is deployed
pub const PERFORMANCE_CONTRACT_ADDRESS: &str = "";
// /\ TODO: this has to be updated once the contract is deployed

pub const NETWORK_MONITORS_CONTRACT_ADDRESS: &str =
    "n1m3a2ltkjqud8mkmrpqvgllrtv2p4r6js6qwl7p8cqkzrq8jg6e2qwqgl8z";
pub const NODE_FAMILIES_CONTRACT_ADDRESS: &str =
    "n1na0vys0z077hq3zrz6pfea85zgv8ks3t5zysdt6y38c87q045hnsyf2g5x";
pub const ECASH_CONTRACT_ADDRESS: &str =
    "n1r7s6aksyc6pqardx88k3rkgfagwvj4z4zum9mmz2sfk3zm2mha0sd4dnun";
pub const GROUP_CONTRACT_ADDRESS: &str =
    "n1e2zq4886zzewpvpucmlw8v9p7zv692f6yck4zjzxh699dkcmlrfqk2knsr";
pub const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1txayqfz5g9qww3rlflpg025xd26m9payz96u54x4fe3s2ktz39xqk67gzx";
pub const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n19604yflqggs9mk2z26mqygq43q2kr3n932egxx630svywd5mpxjsztfpvx";

pub const REWARDING_VALIDATOR_ADDRESS: &str = "n10yyd98e2tuwu0f7ypz9dy3hhjw7v772q6287gy";

pub const NYXD_URL: &str = "https://rpc.nymtech.net";
pub const NYXD_WS: &str = "wss://rpc.nymtech.net/websocket";

// cluster of lite rpc nodes (not part of consensus, aggressive pruning, no archival state)
pub const NYXD_QUERY_LITE: &str = "https://blockstream.nymtech.net";
pub const NYXD_WS_LITE: &str = "wss://blockstream.nymtech.net/websocket";

pub const NYM_API: &str = "https://validator.nymtech.net/api/";
#[cfg(feature = "network")]
pub const NYM_APIS: &[ApiUrlConst] = &[
    ApiUrlConst {
        url: NYM_API,
        front_hosts: None,
    },
    ApiUrlConst {
        url: "https://nym-frontdoor.global.ssl.fastly.net/api/",
        front_hosts: Some(&[
            "fastly-support.global.ssl.fastly.net",
            "yelp.global.ssl.fastly.net",
            "pypi.global.ssl.fastly.net",
        ]),
    },
    ApiUrlConst {
        url: "https://cdn1.media-platform.net/api/",
        front_hosts: None,
    },
];

pub const NYM_VPN_API: &str = "https://nymvpn.com/api/";

pub const UPGRADE_MODE_ATTESTATION_URL: &str =
    "https://nymtech.net/.wellknown/upgrade-mode/attestation.json";
pub const UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY: &str =
    "3bgffBYcfFkTTXc2npNNn9MkddFZ3H2LrPjXDmnJzrqd";

#[cfg(feature = "network")]
pub const NYM_VPN_APIS: &[ApiUrlConst] = &[
    ApiUrlConst {
        url: NYM_VPN_API,
        front_hosts: None,
    },
    ApiUrlConst {
        url: "https://nymvpn-frontdoor.global.ssl.fastly.net/api/",
        front_hosts: Some(&[
            "fastly-support.global.ssl.fastly.net",
            "yelp.global.ssl.fastly.net",
            "pypi.global.ssl.fastly.net",
        ]),
    },
    ApiUrlConst {
        url: "https://edge1.streaming-gateway.com/api/",
        front_hosts: None,
    },
];

#[cfg(feature = "env")]
fn serialize_api_urls(urls: &[ApiUrlConst]) -> String {
    serde_json::to_string(urls)
        .inspect_err(|e| tracing::warn!("failed to serialize nym_api_urls for env: {e}"))
        .unwrap_or_default()
}

// I'm making clippy mad on purpose, because that url HAS TO be updated and deployed before merging
pub const EXIT_POLICY_URL: &str =
    "https://nymtech.net/.wellknown/network-requester/exit-policy.txt";

#[cfg(feature = "network")]
pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        NYXD_URL,
        Some(NYM_API),
        Some(NYXD_WS),
    )]
}

#[cfg(feature = "env")]
const DEFAULT_SUFFIX: &str = "_MAINNET_DEFAULT";

#[cfg(all(feature = "env", feature = "network"))]
fn set_var_to_default(var: &str, value: &str) {
    unsafe {
        std::env::set_var(var, value);
        std::env::set_var(format!("{var}{DEFAULT_SUFFIX}"), "1")
    }
}

#[cfg(all(feature = "env", feature = "network"))]
fn set_var_conditionally_to_default(var: &str, value: &str) {
    if std::env::var(var).is_err() {
        set_var_to_default(var, value)
    }
}

#[cfg(feature = "env")]
pub fn uses_default(var: &str) -> bool {
    std::env::var(format!("{var}{DEFAULT_SUFFIX}")).is_ok()
}

#[cfg(feature = "env")]
pub fn read_var_if_not_default(var: &str) -> Option<String> {
    if uses_default(var) {
        None
    } else {
        std::env::var(var).ok()
    }
}

#[cfg(feature = "env")]
pub fn read_parsed_var_if_not_default<T: std::str::FromStr>(
    var: &str,
) -> Option<Result<T, T::Err>> {
    read_var_if_not_default(var)
        .as_deref()
        .map(std::str::FromStr::from_str)
}

#[cfg(feature = "env")]
pub fn read_parsed_var<T: std::str::FromStr>(var: &str) -> Result<T, T::Err> {
    std::env::var(var).unwrap_or_default().parse()
}

#[cfg(all(feature = "env", feature = "network"))]
pub fn export_to_env() {
    use crate::var_names;

    set_var_to_default(var_names::CONFIGURED, "true");
    set_var_to_default(var_names::NETWORK_NAME, NETWORK_NAME);
    set_var_to_default(var_names::BECH32_PREFIX, BECH32_PREFIX);
    set_var_to_default(var_names::MIX_DENOM, MIX_DENOM.base);
    set_var_to_default(var_names::MIX_DENOM_DISPLAY, MIX_DENOM.display);
    set_var_to_default(var_names::STAKE_DENOM, STAKE_DENOM.base);
    set_var_to_default(var_names::STAKE_DENOM_DISPLAY, STAKE_DENOM.display);
    set_var_to_default(
        var_names::DENOMS_EXPONENT,
        &STAKE_DENOM.display_exponent.to_string(),
    );
    set_var_to_default(var_names::MIXNET_CONTRACT_ADDRESS, MIXNET_CONTRACT_ADDRESS);
    set_var_to_default(
        var_names::VESTING_CONTRACT_ADDRESS,
        VESTING_CONTRACT_ADDRESS,
    );
    set_var_to_default(var_names::ECASH_CONTRACT_ADDRESS, ECASH_CONTRACT_ADDRESS);
    set_var_to_default(var_names::GROUP_CONTRACT_ADDRESS, GROUP_CONTRACT_ADDRESS);
    set_var_to_default(
        var_names::MULTISIG_CONTRACT_ADDRESS,
        MULTISIG_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::COCONUT_DKG_CONTRACT_ADDRESS,
        COCONUT_DKG_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::PERFORMANCE_CONTRACT_ADDRESS,
        PERFORMANCE_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::NETWORK_MONITORS_CONTRACT_ADDRESS,
        NETWORK_MONITORS_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::NODE_FAMILIES_CONTRACT_ADDRESS,
        NODE_FAMILIES_CONTRACT_ADDRESS,
    );
    set_var_to_default(
        var_names::REWARDING_VALIDATOR_ADDRESS,
        REWARDING_VALIDATOR_ADDRESS,
    );
    set_var_to_default(var_names::NYXD, NYXD_URL);
    set_var_to_default(var_names::NYM_API, NYM_API);
    set_var_to_default(var_names::NYM_APIS, &serialize_api_urls(NYM_APIS));
    set_var_to_default(var_names::NYXD_WEBSOCKET, NYXD_WS);
    set_var_to_default(var_names::EXIT_POLICY_URL, EXIT_POLICY_URL);
    set_var_to_default(var_names::NYM_VPN_API, NYM_VPN_API);
    set_var_to_default(var_names::NYM_VPN_APIS, &serialize_api_urls(NYM_VPN_APIS));
    set_var_to_default(
        var_names::UPGRADE_MODE_ATTESTATION_URL,
        UPGRADE_MODE_ATTESTATION_URL,
    );
    set_var_to_default(
        var_names::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY,
        UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY,
    );
    set_var_to_default(var_names::NYXD_QUERY_LITE, NYXD_QUERY_LITE);
    set_var_to_default(var_names::NYXD_WS_LITE, NYXD_WS_LITE);
}

#[cfg(all(feature = "env", feature = "network"))]
pub fn export_to_env_if_not_set() {
    use crate::var_names;

    set_var_conditionally_to_default(var_names::CONFIGURED, "true");
    set_var_conditionally_to_default(var_names::NETWORK_NAME, NETWORK_NAME);
    set_var_conditionally_to_default(var_names::BECH32_PREFIX, BECH32_PREFIX);
    set_var_conditionally_to_default(var_names::MIX_DENOM, MIX_DENOM.base);
    set_var_conditionally_to_default(var_names::MIX_DENOM_DISPLAY, MIX_DENOM.display);
    set_var_conditionally_to_default(var_names::STAKE_DENOM, STAKE_DENOM.base);
    set_var_conditionally_to_default(var_names::STAKE_DENOM_DISPLAY, STAKE_DENOM.display);
    set_var_conditionally_to_default(
        var_names::DENOMS_EXPONENT,
        &STAKE_DENOM.display_exponent.to_string(),
    );
    set_var_conditionally_to_default(var_names::MIXNET_CONTRACT_ADDRESS, MIXNET_CONTRACT_ADDRESS);
    set_var_conditionally_to_default(
        var_names::VESTING_CONTRACT_ADDRESS,
        VESTING_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(var_names::ECASH_CONTRACT_ADDRESS, ECASH_CONTRACT_ADDRESS);
    set_var_conditionally_to_default(var_names::GROUP_CONTRACT_ADDRESS, GROUP_CONTRACT_ADDRESS);
    set_var_conditionally_to_default(
        var_names::MULTISIG_CONTRACT_ADDRESS,
        MULTISIG_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::COCONUT_DKG_CONTRACT_ADDRESS,
        COCONUT_DKG_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::NODE_FAMILIES_CONTRACT_ADDRESS,
        NODE_FAMILIES_CONTRACT_ADDRESS,
    );
    set_var_conditionally_to_default(
        var_names::REWARDING_VALIDATOR_ADDRESS,
        REWARDING_VALIDATOR_ADDRESS,
    );
    set_var_conditionally_to_default(var_names::NYXD, NYXD_URL);
    set_var_conditionally_to_default(var_names::NYM_API, NYM_API);
    set_var_conditionally_to_default(var_names::NYM_APIS, &serialize_api_urls(NYM_APIS));
    set_var_conditionally_to_default(var_names::NYM_VPN_API, NYM_VPN_API);
    set_var_conditionally_to_default(var_names::NYM_VPN_APIS, &serialize_api_urls(NYM_VPN_APIS));
    set_var_conditionally_to_default(var_names::NYXD_WEBSOCKET, NYXD_WS);
    set_var_conditionally_to_default(var_names::EXIT_POLICY_URL, EXIT_POLICY_URL);
    set_var_conditionally_to_default(
        var_names::UPGRADE_MODE_ATTESTATION_URL,
        UPGRADE_MODE_ATTESTATION_URL,
    );
    set_var_conditionally_to_default(
        var_names::UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY,
        UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY,
    );
    set_var_conditionally_to_default(var_names::NYXD_QUERY_LITE, NYXD_QUERY_LITE);
    set_var_conditionally_to_default(var_names::NYXD_WS_LITE, NYXD_WS_LITE);
}

/// Static domain/IP pins used as a DNS fallback when regular resolution of
/// Nym (and Nym-fronting) infrastructure is unavailable or untrustworthy.
#[allow(missing_docs)]
pub mod dns {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    pub const NYM_API_DOMAIN: &str = "validator.nymtech.net";
    pub const NYM_API_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(92, 39, 63, 14))];

    pub const NYM_VPN_API_DOMAIN: &str = "nymvpn.com";
    pub const NYM_VPN_API_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(76, 76, 21, 21))];

    pub const NYM_FRONTDOOR_VERCEL_DOMAIN: &str = "nym-frontdoor.vercel.app";
    pub const NYM_FRONTDOOR_VERCEL_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(64, 29, 17, 195)),
        IpAddr::V4(Ipv4Addr::new(216, 198, 79, 195)),
    ];

    pub const NYM_FRONTDOOR_FASTLY_DOMAIN: &str = "nym-frontdoor.global.ssl.fastly.net";
    pub const NYM_FRONTDOOR_FASTLY_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(151, 101, 193, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 129, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 1, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
    ];

    pub const NYMVPN_FRONTDOOR_FASTLY_DOMAIN: &str = "nymvpn-frontdoor.global.ssl.fastly.net";
    pub const NYMVPN_FRONTDOOR_FASTLY_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(151, 101, 193, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 129, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 1, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
    ];

    pub const YELP_FASTLY_DOMAIN: &str = "yelp.global.ssl.fastly.net";
    pub const YELP_FASTLY_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(151, 101, 193, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 129, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 1, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
    ];

    pub const FASTLY_SUPPORT_DOMAIN: &str = "fastly-support.global.ssl.fastly.net";
    pub const FASTLY_SUPPORT_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(151, 101, 193, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 129, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 1, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
    ];

    pub const PYPI_FASTLY_DOMAIN: &str = "pypi.global.ssl.fastly.net";
    pub const PYPI_FASTLY_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(199, 232, 65, 194)),
        IpAddr::V4(Ipv4Addr::new(151, 101, 65, 194)),
    ];

    pub const VERCEL_APP_DOMAIN: &str = "vercel.app";
    pub const VERCEL_APP_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(64, 29, 17, 195)),
        IpAddr::V4(Ipv4Addr::new(216, 198, 79, 195)),
    ];

    pub const VERCEL_COM_DOMAIN: &str = "vercel.com";
    pub const VERCEL_COM_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(198, 169, 2, 129)),
        IpAddr::V4(Ipv4Addr::new(198, 169, 1, 193)),
    ];

    pub const NYM_API_CDN: &str = "cdn1.media-platform.net";
    pub const NYM_API_CDN_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(172, 104, 178, 252))];

    pub const NYM_VPN_API_EDGE1_STREAMING_GATEWAY_COM: &str = "edge1.streaming-gateway.com";
    pub const NYM_VPN_API_EDGE1_STREAMING_GATEWAY_COM_IPS: &[IpAddr] =
        &[IpAddr::V4(Ipv4Addr::new(139, 162, 57, 231))];

    pub const NYM_COM_DOMAIN: &str = "nym.com";
    pub const NYM_COM_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(76, 76, 21, 22))];

    pub const NYM_STATS_API_DOMAIN: &str = "nym-statistics-api.nymtech.cc";
    pub const NYM_STATS_API_IPS: &[IpAddr] = &[IpAddr::V4(Ipv4Addr::new(185, 19, 29, 32))];

    pub const NYM_RPC_DOMAIN: &str = "rpc.nymtech.net";
    pub const NYM_RPC_IPS: &[IpAddr] = &[
        IpAddr::V4(Ipv4Addr::new(194, 182, 169, 49)),
        IpAddr::V4(Ipv4Addr::new(91, 92, 200, 116)),
        IpAddr::V6(Ipv6Addr::new(
            0x2a04, 0xc43, 0xe00, 0x6f28, 0x400, 0xd8ff, 0xfe00, 0x1483,
        )),
        IpAddr::V6(Ipv6Addr::new(
            0x2a04, 0xc46, 0xe00, 0x6f28, 0x4b3, 0x68ff, 0xfe00, 0x460,
        )),
    ];

    #[allow(unused)]
    pub fn empty_static_addrs() -> HashMap<String, Vec<IpAddr>> {
        HashMap::new()
    }

    #[allow(unused)]
    pub fn default_static_addrs() -> HashMap<String, Vec<IpAddr>> {
        let mut m = HashMap::new();
        m.insert(NYM_API_DOMAIN.to_string(), NYM_API_IPS.to_vec());
        m.insert(NYM_VPN_API_DOMAIN.to_string(), NYM_VPN_API_IPS.to_vec());
        m.insert(
            NYM_FRONTDOOR_VERCEL_DOMAIN.to_string(),
            NYM_FRONTDOOR_VERCEL_IPS.to_vec(),
        );
        m.insert(
            NYM_FRONTDOOR_FASTLY_DOMAIN.to_string(),
            NYM_FRONTDOOR_FASTLY_IPS.to_vec(),
        );
        m.insert(
            NYMVPN_FRONTDOOR_FASTLY_DOMAIN.to_string(),
            NYMVPN_FRONTDOOR_FASTLY_IPS.to_vec(),
        );
        m.insert(YELP_FASTLY_DOMAIN.to_string(), YELP_FASTLY_IPS.to_vec());
        m.insert(PYPI_FASTLY_DOMAIN.to_string(), PYPI_FASTLY_IPS.to_vec());
        m.insert(
            FASTLY_SUPPORT_DOMAIN.to_string(),
            FASTLY_SUPPORT_IPS.to_vec(),
        );
        m.insert(VERCEL_APP_DOMAIN.to_string(), VERCEL_APP_IPS.to_vec());
        m.insert(VERCEL_COM_DOMAIN.to_string(), VERCEL_COM_IPS.to_vec());
        m.insert(NYM_API_CDN.to_string(), NYM_API_CDN_IPS.to_vec());
        m.insert(
            NYM_VPN_API_EDGE1_STREAMING_GATEWAY_COM.to_string(),
            NYM_VPN_API_EDGE1_STREAMING_GATEWAY_COM_IPS.to_vec(),
        );
        m.insert(NYM_COM_DOMAIN.to_string(), NYM_COM_IPS.to_vec());
        m.insert(NYM_STATS_API_DOMAIN.to_string(), NYM_STATS_API_IPS.to_vec());
        m.insert(NYM_RPC_DOMAIN.to_string(), NYM_RPC_IPS.to_vec());
        m
    }
}
