// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, str::FromStr};

use crate::{
    DefaultNetworkDetails, DenomDetails, DenomDetailsOwned, ValidatorDetails, MAINNET_DEFAULTS,
    QA_DEFAULTS, SANDBOX_DEFAULTS,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkDefaultsError {
    #[error("The provided network was invalid")]
    MalformedNetworkProvided(String),
}

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Network {
    QA,
    SANDBOX,
    MAINNET,
}

impl Network {
    fn details(&self) -> &DefaultNetworkDetails {
        match self {
            Self::QA => &QA_DEFAULTS,
            Self::SANDBOX => &SANDBOX_DEFAULTS,
            Self::MAINNET => &MAINNET_DEFAULTS,
        }
    }

    pub fn bech32_prefix(&self) -> &str {
        self.details().bech32_prefix
    }

    pub fn mix_denom(&self) -> &DenomDetails {
        &self.details().mix_denom
    }

    pub fn stake_denom(&self) -> &DenomDetails {
        &self.details().stake_denom
    }

    pub fn mixnet_contract_address(&self) -> &str {
        self.details().mixnet_contract_address
    }

    pub fn vesting_contract_address(&self) -> &str {
        self.details().vesting_contract_address
    }

    pub fn bandwidth_claim_contract_address(&self) -> &str {
        self.details().bandwidth_claim_contract_address
    }

    pub fn coconut_bandwidth_contract_address(&self) -> &str {
        self.details().coconut_bandwidth_contract_address
    }

    pub fn multisig_contract_address(&self) -> &str {
        self.details().multisig_contract_address
    }

    pub fn rewarding_validator_address(&self) -> &str {
        self.details().rewarding_validator_address
    }

    pub fn validators(&self) -> impl Iterator<Item = &ValidatorDetails> {
        self.details().validators.iter()
    }
}

impl FromStr for Network {
    type Err = NetworkDefaultsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "qa" => Ok(Network::QA),
            "sandbox" => Ok(Network::SANDBOX),
            "mainnet" => Ok(Network::MAINNET),
            _ => Err(NetworkDefaultsError::MalformedNetworkProvided(
                s.to_string(),
            )),
        }
    }
}

impl fmt::Display for Network {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Network::QA => f.write_str("QA"),
            Network::SANDBOX => f.write_str("Sandbox"),
            Network::MAINNET => f.write_str("Mainnet"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NetworkDetails {
    bech32_prefix: String,
    mix_denom: DenomDetailsOwned,
    stake_denom: DenomDetailsOwned,
    mixnet_contract_address: String,
    vesting_contract_address: String,
    bandwidth_claim_contract_address: String,
    validators: Vec<ValidatorDetails>,
}

impl From<&DefaultNetworkDetails> for NetworkDetails {
    fn from(details: &DefaultNetworkDetails) -> Self {
        NetworkDetails {
            bech32_prefix: details.bech32_prefix.into(),
            mix_denom: details.mix_denom.into(),
            stake_denom: details.stake_denom.into(),
            mixnet_contract_address: details.mixnet_contract_address.into(),
            vesting_contract_address: details.vesting_contract_address.into(),
            bandwidth_claim_contract_address: details.bandwidth_claim_contract_address.into(),
            validators: details.validators.clone(),
        }
    }
}

impl NetworkDetails {
    pub fn base_mix_denom(&self) -> &str {
        &self.mix_denom.base
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct SupportedNetworks {
    networks: HashMap<Network, NetworkDetails>,
}

impl SupportedNetworks {
    pub fn new(support: Vec<Network>) -> Self {
        SupportedNetworks {
            networks: support
                .into_iter()
                .map(|n| (n, n.details().into()))
                .collect(),
        }
    }

    pub fn bech32_prefix(&self, network: Network) -> Option<&str> {
        self.networks
            .get(&network)
            .map(|network_details| network_details.bech32_prefix.as_str())
    }

    pub fn denom(&self, network: Network) -> Option<&str> {
        self.networks
            .get(&network)
            .map(|network_details| network_details.base_mix_denom())
    }

    pub fn mixnet_contract_address(&self, network: Network) -> Option<&str> {
        self.networks
            .get(&network)
            .map(|network_details| network_details.mixnet_contract_address.as_str())
    }

    pub fn vesting_contract_address(&self, network: Network) -> Option<&str> {
        self.networks
            .get(&network)
            .map(|network_details| network_details.vesting_contract_address.as_str())
    }

    pub fn bandwidth_claim_contract_address(&self, network: Network) -> Option<&str> {
        self.networks
            .get(&network)
            .map(|network_details| network_details.bandwidth_claim_contract_address.as_str())
    }

    pub fn validators(&self, network: Network) -> impl Iterator<Item = &ValidatorDetails> {
        self.networks
            .get(&network)
            .map(|network_details| &network_details.validators)
            .into_iter()
            .flatten()
    }
}
