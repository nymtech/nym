use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contracts addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";
pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
    "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n1ghd753shjuwexxywmgs4xz7x2q732vcn7ty4yw";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str = "n17p9rzwnnfxcjp32un9ug7yhhzgtkhvl988qccs";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "n17p9rzwnnfxcjp32un9ug7yhhzgtkhvl988qccs";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str = "n17p9rzwnnfxcjp32un9ug7yhhzgtkhvl988qccs";
pub(crate) const SERVICE_PROVIDER_DIRECTORY_CONTRACT_ADDRESS: &str =
    "n1ryt076cufyddallg5x0gz3qjz0pd3wg0m4cwkg9njhmlnp6u88qq6nczgj";
pub(crate) const NAME_SERVICE_CONTRACT_ADDRESS: &str =
    "n1cm2u5vfjd3zalfw0p65xyh4tcrw3hjlm0960gzhewga449h4mgas77mjkl";

// -- Constructor functions --

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qwerty-validator.qa.nymte.ch/",
        Some("https://qwerty-validator-api.qa.nymte.ch/api"),
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
            service_provider_directory_contract_address: parse_optional_str(
                SERVICE_PROVIDER_DIRECTORY_CONTRACT_ADDRESS,
            ),
            name_service_contract_address: parse_optional_str(NAME_SERVICE_CONTRACT_ADDRESS),
        },
    }
}
