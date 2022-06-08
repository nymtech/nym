// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    DefaultNetworkDetails, NymNetworkDetails, ValidatorDetails, MAINNET_DEFAULTS, QA_DEFAULTS,
    SANDBOX_DEFAULTS,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, str::FromStr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkDefaultsError {
    #[error("The provided network was invalid")]
    MalformedNetworkProvided(String),
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Network {
    QA,
    SANDBOX,
    MAINNET,
    CUSTOM { details: NymNetworkDetails },
}

impl Network {
    fn details(&self) -> NymNetworkDetails {
        match self {
            Self::QA => (&*QA_DEFAULTS).into(),
            Self::SANDBOX => (&*SANDBOX_DEFAULTS).into(),
            Self::MAINNET => (&*MAINNET_DEFAULTS).into(),
            // I dislike the clone here, but for compatibility reasons we cannot define other networks with `NymNetworkDetails` directly yet
            Self::CUSTOM { details } => details.clone(),
        }
    }

    pub fn bech32_prefix(&self) -> String {
        self.details().chain_details.bech32_account_prefix
    }

    #[deprecated(note = "please use mix_denom instead")]
    pub fn denom(&self) -> String {
        self.mix_denom()
    }

    pub fn mix_denom(&self) -> String {
        self.details().chain_details.mix_denom
    }

    pub fn stake_denom(&self) -> String {
        self.details().chain_details.stake_denom
    }

    pub fn mixnet_contract_address(&self) -> Option<String> {
        self.details().contracts.mixnet_contract_address
    }

    pub fn vesting_contract_address(&self) -> Option<String> {
        self.details().contracts.vesting_contract_address
    }

    pub fn bandwidth_claim_contract_address(&self) -> Option<String> {
        self.details().contracts.bandwidth_claim_contract_address
    }

    pub fn coconut_bandwidth_contract_address(&self) -> Option<String> {
        self.details().contracts.coconut_bandwidth_contract_address
    }

    pub fn multisig_contract_address(&self) -> Option<String> {
        self.details().contracts.multisig_contract_address
    }

    pub fn validators(&self) -> Vec<ValidatorDetails> {
        self.details().endpoints
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
            Network::CUSTOM { .. } => f.write_str("Custom"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct NetworkDetails {
    bech32_prefix: String,
    denom: String,
    mixnet_contract_address: String,
    vesting_contract_address: String,
    bandwidth_claim_contract_address: String,
    validators: Vec<ValidatorDetails>,
}

impl From<&DefaultNetworkDetails<'_>> for NetworkDetails {
    fn from(details: &DefaultNetworkDetails<'_>) -> Self {
        NetworkDetails {
            bech32_prefix: details.bech32_prefix.into(),
            denom: details.denom.into(),
            mixnet_contract_address: details.mixnet_contract_address.into(),
            vesting_contract_address: details.vesting_contract_address.into(),
            bandwidth_claim_contract_address: details.bandwidth_claim_contract_address.into(),
            validators: details.validators.clone(),
        }
    }
}

// this also has to exist for compatibility reasons since I don't want to be touching the wallet now
impl From<NymNetworkDetails> for NetworkDetails {
    fn from(details: NymNetworkDetails) -> Self {
        NetworkDetails {
            bech32_prefix: details.chain_details.bech32_account_prefix,
            denom: details.chain_details.mix_denom,
            mixnet_contract_address: details
                .contracts
                .mixnet_contract_address
                .unwrap_or_default(),
            vesting_contract_address: details
                .contracts
                .vesting_contract_address
                .unwrap_or_default(),
            bandwidth_claim_contract_address: details
                .contracts
                .bandwidth_claim_contract_address
                .unwrap_or_default(),
            validators: details.endpoints,
        }
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
                .map(|n| {
                    let details = n.details().into();
                    (n, details)
                })
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
            .map(|network_details| network_details.denom.as_str())
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
