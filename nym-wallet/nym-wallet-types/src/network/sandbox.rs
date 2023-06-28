use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const BECH32_PREFIX: &str = "nymt";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unymt", "nymt", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyxt", "nyxt", 6);

// -- Contract addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "nymt1ghd753shjuwexxywmgs4xz7x2q732vcnstz02j";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "nymt14ejqjyq8um4p3xfqj74yld5waqljf88fn549lh";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "nymt1nz0r0au8aj6dc00wmm3ufy4g4k86rjzlgq608r";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str = "nymt1k8re7jwz6rnnwrktnejdwkwnncte7ek7kk6fvg";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "nymt1k8re7jwz6rnnwrktnejdwkwnncte7ek7kk6fvg";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str = "nymt1k8re7jwz6rnnwrktnejdwkwnncte7ek7kk6fvg";
pub(crate) const EPHEMERA_CONTRACT_ADDRESS: &str = "nymt1k8re7jwz6rnnwrktnejdwkwnncte7ek7kk6fvg";

// -- Constructor functions --

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://sandbox-validator.nymtech.net",
        Some("https://sandbox-validator.nymtech.net/api"),
    )]
}

pub(crate) fn network_details() -> nym_network_defaults::NymNetworkDetails {
    nym_network_defaults::NymNetworkDetails {
        chain_details: ChainDetails {
            bech32_account_prefix: BECH32_PREFIX.to_string(),
            mix_denom: MIX_DENOM.into(),
            stake_denom: STAKE_DENOM.into(),
        },
        endpoints: validators(),
        contracts: NymContracts {
            mixnet_contract_address: parse_optional_str(MIXNET_CONTRACT_ADDRESS),
            vesting_contract_address: parse_optional_str(VESTING_CONTRACT_ADDRESS),
            bandwidth_claim_contract_address: parse_optional_str(BANDWIDTH_CLAIM_CONTRACT_ADDRESS),
            coconut_bandwidth_contract_address: parse_optional_str(
                COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
            ),
            group_contract_address: parse_optional_str(GROUP_CONTRACT_ADDRESS),
            multisig_contract_address: parse_optional_str(MULTISIG_CONTRACT_ADDRESS),
            coconut_dkg_contract_address: parse_optional_str(COCONUT_DKG_CONTRACT_ADDRESS),
            ephemera_contract_address: parse_optional_str(EPHEMERA_CONTRACT_ADDRESS),
            service_provider_directory_contract_address: None,
            name_service_contract_address: None,
        },
    }
}
