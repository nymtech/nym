use super::parse_optional_str;
use nym_network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

// -- Chain details --

pub(crate) const NETWORK_NAME: &str = "qa";

pub(crate) const BECH32_PREFIX: &str = "n";
pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

// -- Contract addresses --

pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
    "n1hm4y6fzgxgu688jgf7ek66px6xkrtmn3gyk8fax3eawhp68c2d5qujz296";
pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
    "n1jlzdxnyces4hrhqz68dqk28mrw5jgwtcfq0c2funcwrmw0dx9l9s8nnnvj";
pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
    "n1w798gp0zqv3s9hjl3jlnwxtwhykga6rn93p46q2crsdqhaj3y4gs68f74j";
pub(crate) const GROUP_CONTRACT_ADDRESS: &str =
    "n1sthrn5ep8ls5vzz8f9gp89khhmedahhdqd244dh9uqzk3hx2pzrsvf7zgk";
pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
    "n1sr06m8yqg0wzqqyqvzvp5t07dj4nevx9u8qc7j4qa72qu8e3ct8qledthy";
pub(crate) const COCONUT_DKG_CONTRACT_ADDRESS: &str =
    "n1udfs22xpxle475m2nz7u47jfa3vngncdegmczwwdx00cmetypa3s7uyuqn";

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
        },
        explorer_api: parse_optional_str(EXPLORER_API),
        nym_vpn_api_url: None,
    }
}
