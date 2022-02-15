// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{mainnet, qa, sandbox, ValidatorDetails};

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Network {
    QA,
    SANDBOX,
    MAINNET,
}

impl Network {
    pub fn bech32_prefix(&self) -> String {
        match self {
            Self::QA => String::from(qa::BECH32_PREFIX),
            Self::SANDBOX => String::from(sandbox::BECH32_PREFIX),
            Self::MAINNET => String::from(mainnet::BECH32_PREFIX),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NetworkDetails {
    bech32_prefix: String,
    denom: String,
    mixnet_contract_address: String,
    vesting_contract_address: String,
    bandwidth_claim_contract_address: String,
    rewarding_validator_address: String,
    validators: Vec<ValidatorDetails>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SupportedNetworks {
    networks: HashMap<Network, NetworkDetails>,
}

impl SupportedNetworks {
    pub fn new(support: Vec<Network>) -> Self {
        let mut networks = HashMap::new();

        for network in support {
            match network {
                Network::MAINNET => networks.insert(
                    Network::MAINNET,
                    NetworkDetails {
                        bech32_prefix: String::from(mainnet::BECH32_PREFIX),
                        denom: String::from(mainnet::DENOM),
                        mixnet_contract_address: String::from(mainnet::MIXNET_CONTRACT_ADDRESS),
                        vesting_contract_address: String::from(mainnet::VESTING_CONTRACT_ADDRESS),
                        bandwidth_claim_contract_address: String::from(
                            mainnet::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
                        ),
                        rewarding_validator_address: String::from(
                            mainnet::REWARDING_VALIDATOR_ADDRESS,
                        ),
                        validators: mainnet::validators(),
                    },
                ),

                Network::SANDBOX => networks.insert(
                    Network::SANDBOX,
                    NetworkDetails {
                        bech32_prefix: String::from(sandbox::BECH32_PREFIX),
                        denom: String::from(sandbox::DENOM),
                        mixnet_contract_address: String::from(sandbox::MIXNET_CONTRACT_ADDRESS),
                        vesting_contract_address: String::from(sandbox::VESTING_CONTRACT_ADDRESS),
                        bandwidth_claim_contract_address: String::from(
                            sandbox::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
                        ),
                        rewarding_validator_address: String::from(
                            sandbox::REWARDING_VALIDATOR_ADDRESS,
                        ),
                        validators: sandbox::validators(),
                    },
                ),
                Network::QA => networks.insert(
                    Network::QA,
                    NetworkDetails {
                        bech32_prefix: String::from(qa::BECH32_PREFIX),
                        denom: String::from(qa::DENOM),
                        mixnet_contract_address: String::from(qa::MIXNET_CONTRACT_ADDRESS),
                        vesting_contract_address: String::from(qa::VESTING_CONTRACT_ADDRESS),
                        bandwidth_claim_contract_address: String::from(
                            qa::BANDWIDTH_CLAIM_CONTRACT_ADDRESS,
                        ),
                        rewarding_validator_address: String::from(qa::REWARDING_VALIDATOR_ADDRESS),
                        validators: qa::validators(),
                    },
                ),
            };
        }
        SupportedNetworks { networks }
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

    pub fn rewarding_validator_address(&self, network: Network) -> Option<&str> {
        self.networks
            .get(&network)
            .map(|network_details| network_details.rewarding_validator_address.as_str())
    }

    pub fn validators(&self, network: Network) -> Option<&Vec<ValidatorDetails>> {
        self.networks
            .get(&network)
            .map(|network_details| &network_details.validators)
    }
}
