use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const NETWORK_NAME: &str = "qa";

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contract addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n1khq8f8vhah0gtljahrnsr3utl5lrhlf0xafs6pkvetnyumv7vt4qxh2ckx";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1jlzdxnyces4hrhqz68dqk28mrw5jgwtcfq0c2funcwrmw0dx9l9s8nnnvj";
pub(crate) const ECASH_CONTRACT_ADDRESS: &str =
    "n13xspq62y9gq6nueqmywxcdv2yep4p6nzv98w2889k25v3nhdy2dq2rkrk7";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str =
    "n13l7rwuwktklrwskc7m6lv70zws07en85uma28j7dxwsz9y5hvvhspl7a2t";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
    "n138c9pyf7f3hyx0j3t6vmsz7ultnw2wj0lu6hzndep9z5grgq9haqlc25k0";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1pk8jgr6y4c5k93gz7qf3xc0hvygmp7csk88c2tf8l39tkq6834wq2a6dtr";

// -- Constructor functions --

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://benny-validator.qa.nymte.ch/",
        Some("https://qa-nym-api.qa.nymte.ch/api"),
        Some("wss://qa-validator.qa.nymte.ch/websocket"),
    )]
}

pub(crate) const EXPLORER_API: &str = "https://qa-network-explorer.qa.nymte.ch/api/";

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
