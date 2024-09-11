use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const NETWORK_NAME: &str = "sandbox";

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contract addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1unyuj8qnmygvzuex3dwmg9yzt9alhvyeat0uu0jedg2wj33efl5qackslz";
pub(crate) const ECASH_CONTRACT_ADDRESS: &str =
    "n1v3vydvs2ued84yv3khqwtgldmgwn0elljsdh08dr5s2j9x4rc5fs9jlwz9";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str =
    "n1ewmwz97xm0h8rdk8sw7h9mwn866qkx9hl9zlmagqfkhuzvwk5hhq844ue9";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1tz0setr8vkh9udp8xyxgpqc89ns27k4d0jx2h942hr0ax63yjhmqz6xct8";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1v3n2ly2dp3a9ng3ff6rh26yfkn0pc5hed7w2shc5u9ca5c865utqj5elvh";

// -- Constructor functions --

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://rpc.sandbox.nymtech.net",
        Some("https://sandbox-nym-api1.nymtech.net/api"),
        Some("wss://rpc.sandbox.nymtech.net/websocket"),
    )]
}

pub(crate) const EXPLORER_API: &str = "https://sandbox-explorer.nymtech.net/api/";

pub(crate) fn network_details() -> nym_network_defaults::NymNetworkDetails {
    nym_network_defaults::NymNetworkDetails {
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
            ecash_contract_address: parse_optional_str(ECASH_CONTRACT_ADDRESS),
            group_contract_address: parse_optional_str(GROUP_CONTRACT_ADDRESS),
            multisig_contract_address: parse_optional_str(MULTISIG_CONTRACT_ADDRESS),
            coconut_dkg_contract_address: parse_optional_str(COCONUT_DKG_CONTRACT_ADDRESS),
        },
        explorer_api: parse_optional_str(EXPLORER_API),
        nym_vpn_api_url: None,
    }
}
