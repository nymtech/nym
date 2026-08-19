// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "network")]
use crate::{
    ApiUrlConst, ChainDetails, DenomDetails, NymContracts, NymNetworkDetails, ValidatorDetails,
};
#[cfg(feature = "network")]
use std::ops::Not;

pub const NETWORK_NAME: &str = "sandbox";

pub const BECH32_PREFIX: &str = "n";

#[cfg(feature = "network")]
pub const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
#[cfg(feature = "network")]
pub const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contract addresses --

pub const MIXNET_CONTRACT_ADDRESS: &str =
    "n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav";
pub const VESTING_CONTRACT_ADDRESS: &str =
    "n1unyuj8qnmygvzuex3dwmg9yzt9alhvyeat0uu0jedg2wj33efl5qackslz";
pub const ECASH_CONTRACT_ADDRESS: &str =
    "n1v3vydvs2ued84yv3khqwtgldmgwn0elljsdh08dr5s2j9x4rc5fs9jlwz9";
pub const GROUP_CONTRACT_ADDRESS: &str =
    "n1ewmwz97xm0h8rdk8sw7h9mwn866qkx9hl9zlmagqfkhuzvwk5hhq844ue9";
pub const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1tz0setr8vkh9udp8xyxgpqc89ns27k4d0jx2h942hr0ax63yjhmqz6xct8";
pub const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1v3n2ly2dp3a9ng3ff6rh26yfkn0pc5hed7w2shc5u9ca5c865utqj5elvh";

// \/ TODO: this has to be updated once the contract is deployed
pub const PERFORMANCE_CONTRACT_ADDRESS: &str = "";
// /\ TODO: this has to be updated once the contract is deployed

pub const NETWORK_MONITORS_CONTRACT_ADDRESS: &str =
    "n1x5krtvyqklj360x38v62ze42g8s8trfsfqzlv8c9296chcpvqadssqnem5";

pub const NODE_FAMILIES_CONTRACT_ADDRESS: &str =
    "n13clyapdqk5umyynp20kqwf59rxlwlp24yf2ltzasflhsdhrxq7fsahyr6z";

pub const NYXD_URL: &str = "https://validator-sandbox-1.nymtech.net";
pub const NYXD_WS: &str = "wss://validator-sandbox-1.nymtech.net/websocket";

pub const NYXD_QUERY_LITE: &str = "https://validator-sandbox-1.nymtech.net";
pub const NYXD_WS_LITE: &str = "wss://validator-sandbox-1.nymtech.net/websocket";

pub const UPGRADE_MODE_ATTESTATION_URL: &str =
    "http://upgrade-mode.performance.nymte.ch/.wellknown/sandbox/attestation.json";
pub const UPGRADE_MODE_ATTESTER_ED25519_BS58_PUBKEY: &str =
    "EGwzKXPrqStv8cHF68VT2LbQuEBGDPzhCAixScvybfem";

pub const NYM_VPN_API: &str =
    "https://nym-vpn-api-git-deploy-sandbox-nyx-network-staging.vercel.app/api/";

#[cfg(feature = "network")]
pub const NYM_VPN_APIS: &[ApiUrlConst] = &[
    ApiUrlConst {
        url: NYM_VPN_API,
        front_hosts: Some(&["vercel.app", "vercel.com"]),
    },
    ApiUrlConst {
        url: "https://nym-frontdoor.vercel.app/sandbox/nym-vpn-api/",
        front_hosts: Some(&["vercel.app", "vercel.com"]),
    },
];

#[cfg(feature = "network")]
pub const NYM_APIS: &[ApiUrlConst] = &[
    ApiUrlConst {
        url: "https://sandbox-nym-api1.nymtech.net/api/",
        front_hosts: None,
    },
    ApiUrlConst {
        url: "https://nym-frontdoor.vercel.app/sandbox/nym-api/",
        front_hosts: Some(&["vercel.app", "vercel.com"]),
    },
];

pub const EXIT_POLICY_URL: &str =
    "https://nymtech.net/.wellknown/network-requester/exit-policy.txt";

#[cfg(feature = "network")]
pub fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://validator-sandbox-1.nymtech.net",
        Some("https://sandbox-nym-api1.nymtech.net/api"),
        Some("wss://validator-sandbox-1.nymtech.net/websocket"),
    )]
}

#[cfg(feature = "network")]
pub fn network_details() -> NymNetworkDetails {
    NymNetworkDetails {
        network_name: NETWORK_NAME.into(),
        chain_details: ChainDetails {
            bech32_account_prefix: BECH32_PREFIX.to_string(),
            mix_denom: MIX_DENOM.into(),
            stake_denom: STAKE_DENOM.into(),
        },
        endpoints: validators(),
        contracts: NymContracts {
            mixnet_contract_address: parse_optional_str(MIXNET_CONTRACT_ADDRESS),
            vesting_contract_address: parse_optional_str(VESTING_CONTRACT_ADDRESS),
            performance_contract_address: parse_optional_str(PERFORMANCE_CONTRACT_ADDRESS),
            network_monitors_contract_address: parse_optional_str(
                NETWORK_MONITORS_CONTRACT_ADDRESS,
            ),
            node_families_contract_address: parse_optional_str(NODE_FAMILIES_CONTRACT_ADDRESS),
            ecash_contract_address: parse_optional_str(ECASH_CONTRACT_ADDRESS),
            group_contract_address: parse_optional_str(GROUP_CONTRACT_ADDRESS),
            multisig_contract_address: parse_optional_str(MULTISIG_CONTRACT_ADDRESS),
            coconut_dkg_contract_address: parse_optional_str(COCONUT_DKG_CONTRACT_ADDRESS),
        },
        nym_api_urls: Some(NYM_APIS.iter().copied().map(Into::into).collect()),
        nym_vpn_api_urls: Some(NYM_VPN_APIS.iter().copied().map(Into::into).collect()),
        nym_vpn_api_url: parse_optional_str(NYM_VPN_API),
    }
}

#[cfg(feature = "network")]
fn parse_optional_str(raw: &str) -> Option<String> {
    raw.is_empty().not().then(|| raw.into())
}
