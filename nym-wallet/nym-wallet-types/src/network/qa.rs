use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const NETWORK_NAME: &str = "qa";

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contract addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n10qt8wg0n7z740ssvf3urmvgtjhxpyp74hxqvqt7z226gykuus7eq5u9pvq";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1vguuxez2h5ekltfj9gjd62fs5k4rl2zy5hfrncasykzw08rezpfstk9xtk";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n1ghd753shjuwexxywmgs4xz7x2q732vcn7ty4yw";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str =
    "n1g4xlpqy29m50j5y69reguae328tc9y83l4299pf2wmjn0xczq5js3704ql";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1p54qvfde6mpnqvz3dnpa78x2qyyr5k4sgw9qr97mxjgklc5gze9sv6t964";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1xqkp8x4gqwjnhemtemc5dqhwll6w6rrgpywvhka7sh8vz8swul9sp3lv3w";
pub(crate) const SERVICE_PROVIDER_DIRECTORY_CONTRACT_ADDRESS: &str =
    "n1nhdr07kmjns2x8dnp53tdk4qxreze8zdxj6xucyvkdj9tta73rjqa96wps";
pub(crate) const NAME_SERVICE_CONTRACT_ADDRESS: &str =
    "n1uz24lsnwxvhep8m3gjec7ev86twlhlqrf5rphlgn3rda3zu048ssjqr5w9";

// -- Constructor functions --

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qa-validator.qa.nymte.ch/",
        Some("https://qa-nym-api.qa.nymte.ch/api"),
    )]
}

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
            service_provider_directory_contract_address: parse_optional_str(
                SERVICE_PROVIDER_DIRECTORY_CONTRACT_ADDRESS,
            ),
            name_service_contract_address: parse_optional_str(NAME_SERVICE_CONTRACT_ADDRESS),
        },
    }
}
