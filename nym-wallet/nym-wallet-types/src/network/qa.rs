use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const NETWORK_NAME: &str = "qa";

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contract addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n14hj2tavq8fpesdwxxcu44rty3hh90vhujrvcmstl4zr3txmfvw9sjyvg3g";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1nc5tatafv6eyq7llkr2gv50ff9e22mnf70qgjlv737ktmt4eswrq73f2nw";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n1w798gp0zqv3s9hjl3jlnwxtwhykga6rn93p46q2crsdqhaj3y4gs68f74j";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str =
    "n1sthrn5ep8ls5vzz8f9gp89khhmedahhdqd244dh9uqzk3hx2pzrsvf7zgk";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1sr06m8yqg0wzqqyqvzvp5t07dj4nevx9u8qc7j4qa72qu8e3ct8qledthy";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1udfs22xpxle475m2nz7u47jfa3vngncdegmczwwdx00cmetypa3s7uyuqn";
pub(crate) const SERVICE_PROVIDER_DIRECTORY_CONTRACT_ADDRESS: &str =
    "n13ehuhysn5mqjeaheeuew2gjs785f6k7jm8vfsqg3jhtpkwppcmzq6m2hmz";
pub(crate) const NAME_SERVICE_CONTRACT_ADDRESS: &str =
    "n1qum2tr7hh4y7ruzew68c64myjec0dq2s2njf6waja5t0w879lutqadamme";

// -- Constructor functions --

pub(crate) fn validators() -> Vec<ValidatorDetails> {
    vec![ValidatorDetails::new(
        "https://qa-validator.qa.nymte.ch/",
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
        explorer_api: parse_optional_str(EXPLORER_API),
    }
}
