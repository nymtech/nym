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
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n16a32stm6kknhq5cc8rx77elr66pygf2hfszw7wvpq746x3uffylqkjar4l";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str =
    "n1pd7kfgvr5tpcv0xnlv46c4jsq9jg2r799xxrcwqdm4l2jhq2pjwqrmz5ju";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
    "n14ph4e660eyqz0j36zlkaey4zgzexm5twkmjlqaequxr2cjm9eprqsmad6k";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1ahg0erc2fs6xx3j5m8sfx3ryuzdjh6kf6qm9plsf865fltekyrfsesac6a";

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
            coconut_bandwidth_contract_address: parse_optional_str(
                COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
            ),
            group_contract_address: parse_optional_str(GROUP_CONTRACT_ADDRESS),
            multisig_contract_address: parse_optional_str(MULTISIG_CONTRACT_ADDRESS),
            coconut_dkg_contract_address: parse_optional_str(COCONUT_DKG_CONTRACT_ADDRESS),
        },
        explorer_api: parse_optional_str(EXPLORER_API),
        nym_vpn_api_url: None,
    }
}
