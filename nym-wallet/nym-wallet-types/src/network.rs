// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::{mainnet, DenomDetails, NymNetworkDetails};
use nym_types::{currency::DecCoin, error::TypesError};
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Not, str::FromStr};
use strum::EnumIter;

#[allow(clippy::upper_case_acronyms)]
#[cfg_attr(feature = "generate-ts", derive(ts_rs::TS))]
#[cfg_attr(
    feature = "generate-ts",
    ts(export_to = "nym-wallet/src/types/rust/Network.ts")
)]
#[derive(Copy, Clone, Debug, Deserialize, EnumIter, Eq, Hash, PartialEq, Serialize)]
pub enum Network {
    QA,
    SANDBOX,
    MAINNET,
}

impl Network {
    pub fn as_key(&self) -> String {
        self.to_string().to_lowercase()
    }

    pub fn mix_denom(&self) -> DenomDetails {
        match self {
            Network::QA => qa::MIX_DENOM,
            Network::SANDBOX => sandbox::MIX_DENOM,
            Network::MAINNET => mainnet::MIX_DENOM,
        }
    }

    pub fn base_mix_denom(&self) -> &str {
        match self {
            Network::QA => qa::MIX_DENOM.base,
            Network::SANDBOX => sandbox::MIX_DENOM.base,
            Network::MAINNET => mainnet::MIX_DENOM.base,
        }
    }

    pub fn display_mix_denom(&self) -> &str {
        match self {
            Network::QA => qa::MIX_DENOM.display,
            Network::SANDBOX => sandbox::MIX_DENOM.display,
            Network::MAINNET => mainnet::MIX_DENOM.display,
        }
    }

    pub fn default_zero_mix_display_coin(&self) -> DecCoin {
        DecCoin::zero(self.display_mix_denom())
    }
}

impl Default for Network {
    fn default() -> Self {
        Network::MAINNET
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<Network> for NymNetworkDetails {
    fn from(network: Network) -> Self {
        match network {
            Network::QA => qa::network_details(),
            Network::SANDBOX => sandbox::network_details(),
            Network::MAINNET => NymNetworkDetails::new_mainnet(),
        }
    }
}

impl FromStr for Network {
    type Err = TypesError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qa" => Ok(Network::QA),
            "sandbox" => Ok(Network::SANDBOX),
            "mainnet" => Ok(Network::MAINNET),
            _ => Err(TypesError::UnknownNetwork(s.to_string())),
        }
    }
}

fn parse_optional_str(raw: &str) -> Option<String> {
    raw.is_empty().not().then(|| raw.into())
}

mod sandbox {
    use network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

    use super::parse_optional_str;

    pub(crate) const BECH32_PREFIX: &str = "nymt";

    pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unymt", "nymt", 6);
    pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyxt", "nyxt", 6);

    pub(crate) const MIXNET_CONTRACT_ADDRESS: &str = "nymt1ghd753shjuwexxywmgs4xz7x2q732vcnstz02j";
    pub(crate) const VESTING_CONTRACT_ADDRESS: &str = "nymt14ejqjyq8um4p3xfqj74yld5waqljf88fn549lh";
    pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
        "nymt17p9rzwnnfxcjp32un9ug7yhhzgtkhvl9f8xzkv";
    pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
        "nymt1nz0r0au8aj6dc00wmm3ufy4g4k86rjzlgq608r";
    pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str =
        "nymt1k8re7jwz6rnnwrktnejdwkwnncte7ek7kk6fvg";
    pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
        hex_literal::hex!("8e0DcFF7F3085235C32E845f3667aEB3f1e83133");
    pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
        hex_literal::hex!("E8883BAeF3869e14E4823F46662e81D4F7d2A81F");
    //pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str =
    //"nymt1jh0s6qu6tuw9ut438836mmn7f3f2wencrnmdj4";

    //pub(crate) const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "http://0.0.0.0";
    pub(crate) fn validators() -> Vec<ValidatorDetails> {
        vec![ValidatorDetails::new(
            "https://sandbox-validator.nymtech.net",
            Some("https://sandbox-validator.nymtech.net/api"),
        )]
    }

    pub(crate) fn network_details() -> network_defaults::NymNetworkDetails {
        network_defaults::NymNetworkDetails {
            chain_details: ChainDetails {
                bech32_account_prefix: BECH32_PREFIX.to_string(),
                mix_denom: MIX_DENOM.into(),
                stake_denom: STAKE_DENOM.into(),
            },
            endpoints: validators(),
            contracts: NymContracts {
                mixnet_contract_address: parse_optional_str(MIXNET_CONTRACT_ADDRESS),
                vesting_contract_address: parse_optional_str(VESTING_CONTRACT_ADDRESS),
                bandwidth_claim_contract_address: parse_optional_str(
                    BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
                ),
                coconut_bandwidth_contract_address: parse_optional_str(
                    COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
                ),
                multisig_contract_address: parse_optional_str(MULTISIG_CONTRACT_ADDRESS),
            },
        }
    }
}

mod qa {
    use network_defaults::{ChainDetails, DenomDetails, NymContracts, ValidatorDetails};

    use super::parse_optional_str;

    pub(crate) const BECH32_PREFIX: &str = "n";

    pub(crate) const MIX_DENOM: DenomDetails = DenomDetails::new("unym", "nym", 6);
    pub(crate) const STAKE_DENOM: DenomDetails = DenomDetails::new("unyx", "nyx", 6);

    pub(crate) const MIXNET_CONTRACT_ADDRESS: &str =
        "n1rjzps6qrmdqmf0xz4cn4x4rcmqeqzq6hnzqg4wcvd0r2lyasdq5sepn5s8";
    pub(crate) const VESTING_CONTRACT_ADDRESS: &str =
        "n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav";
    pub(crate) const BANDWIDTH_CLAIM_CONTRACT_ADDRESS: &str =
        "n19lc9u84cz0yz3fww5283nucc9yvr8gsjmgeul0";
    pub(crate) const COCONUT_BANDWIDTH_CONTRACT_ADDRESS: &str =
        "n1ghd753shjuwexxywmgs4xz7x2q732vcn7ty4yw";
    pub(crate) const MULTISIG_CONTRACT_ADDRESS: &str = "n17p9rzwnnfxcjp32un9ug7yhhzgtkhvl988qccs";
    pub(crate) const _ETH_CONTRACT_ADDRESS: [u8; 20] =
        hex_literal::hex!("0000000000000000000000000000000000000000");
    pub(crate) const _ETH_ERC20_CONTRACT_ADDRESS: [u8; 20] =
        hex_literal::hex!("0000000000000000000000000000000000000000");
    //pub(crate) const REWARDING_VALIDATOR_ADDRESS: &str = "n1tfzd4qz3a45u8p4mr5zmzv66457uwjgcl05jdq";

    //pub(crate) const STATISTICS_SERVICE_DOMAIN_ADDRESS: &str = "http://0.0.0.0";
    pub(crate) fn validators() -> Vec<ValidatorDetails> {
        vec![ValidatorDetails::new(
            "https://qa-validator.nymtech.net",
            Some("https://qa-validator-api.nymtech.net/api"),
        )]
    }

    pub(crate) fn network_details() -> network_defaults::NymNetworkDetails {
        network_defaults::NymNetworkDetails {
            chain_details: ChainDetails {
                bech32_account_prefix: BECH32_PREFIX.to_string(),
                mix_denom: MIX_DENOM.into(),
                stake_denom: STAKE_DENOM.into(),
            },
            endpoints: validators(),
            contracts: NymContracts {
                mixnet_contract_address: parse_optional_str(MIXNET_CONTRACT_ADDRESS),
                vesting_contract_address: parse_optional_str(VESTING_CONTRACT_ADDRESS),
                bandwidth_claim_contract_address: parse_optional_str(
                    BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
                ),
                coconut_bandwidth_contract_address: parse_optional_str(
                    COCONUT_BANDWIDTH_CONTRACT_ADDRESS,
                ),
                multisig_contract_address: parse_optional_str(MULTISIG_CONTRACT_ADDRESS),
            },
        }
    }
}
