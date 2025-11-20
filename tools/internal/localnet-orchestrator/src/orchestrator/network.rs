// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::contract_build_names;
use crate::orchestrator::account::Account;
use crate::orchestrator::cosmwasm_contract::{ContractBeingInitialised, CosmwasmContract};
use anyhow::{Context, bail};
use nym_config::defaults::{ApiUrl, ChainDetails, NymNetworkDetails, ValidatorDetails};
use serde::{Deserialize, Serialize};
use std::path::Path;
use url::Url;

pub(crate) struct Localnet {
    pub(crate) human_name: String,

    pub(crate) nyxd: Option<NyxdDetails>,

    pub(crate) nym_api_endpoint: Option<Url>,

    pub(crate) contracts: Option<NymContracts>,

    pub(crate) auxiliary_accounts: Option<AuxiliaryAccounts>,
}

impl Localnet {
    pub(crate) fn new(human_name: String) -> Self {
        Localnet {
            human_name,
            nyxd: None,
            nym_api_endpoint: None,
            contracts: None,
            auxiliary_accounts: None,
        }
    }

    /// Best effort conversion of `Localnet` information into `NymNetworkDetails`
    /// The result will depend on the current state of localnet setup, e.g.
    /// if contracts have not yet been initialised, the relevant addresses will not be set.
    pub(crate) fn nym_network_details(&self) -> anyhow::Result<NymNetworkDetails> {
        let mut details = NymNetworkDetails::new_empty();
        details.network_name = "localnet".to_string();

        let mut validator_details =
            ValidatorDetails::new_nyxd_only(self.localhost_rpc_endpoint()?.to_string());

        // localnet uses the same chain-details (i.e. denoms, prefixes) as mainnet
        details.chain_details = ChainDetails::mainnet();

        if let Some(contracts) = self.contracts.as_ref() {
            details.contracts.mixnet_contract_address = Some(contracts.mixnet.address.to_string());
            details.contracts.vesting_contract_address =
                Some(contracts.vesting.address.to_string());
            details.contracts.performance_contract_address =
                Some(contracts.performance.address.to_string());
            details.contracts.ecash_contract_address = Some(contracts.ecash.address.to_string());
            details.contracts.group_contract_address =
                Some(contracts.cw4_group.address.to_string());
            details.contracts.multisig_contract_address =
                Some(contracts.cw3_multisig.address.to_string());
            details.contracts.coconut_dkg_contract_address =
                Some(contracts.dkg.address.to_string());
        }

        if let Some(nym_api) = self.nym_api_endpoint.as_ref() {
            validator_details.api_url = Some(nym_api.to_string());
            details.nym_api_urls = Some(vec![ApiUrl {
                url: nym_api.to_string(),
                front_hosts: None,
            }])
        }

        details.endpoints = vec![validator_details];
        Ok(details)
    }

    pub(crate) fn env_file_content(&self) -> anyhow::Result<String> {
        let mut env_content = r#"
CONFIGURED=true

RUST_LOG=info
RUST_BACKTRACE=1
NETWORK_NAME=localnet
BECH32_PREFIX=n
MIX_DENOM=unym
MIX_DENOM_DISPLAY=nym
STAKE_DENOM=unyx
STAKE_DENOM_DISPLAY=nyx
DENOMS_EXPONENT=6

"#
        .to_string();

        if let Some(contracts) = &self.contracts {
            // if contracts are defined so must be the addresses
            let aux = self.auxiliary_accounts()?;

            env_content.push_str(&format!(
                r#"REWARDING_VALIDATOR_ADDRESS={}
MIXNET_CONTRACT_ADDRESS={}
VESTING_CONTRACT_ADDRESS={}
GROUP_CONTRACT_ADDRESS={}
MULTISIG_CONTRACT_ADDRESS={}
COCONUT_DKG_CONTRACT_ADDRESS={}
ECASH_CONTRACT_ADDRESS={}
PERFORMANCE_CONTRACT_ADDRESS={}

"#,
                aux.mixnet_rewarder.address,
                contracts.mixnet.address,
                contracts.vesting.address,
                contracts.cw4_group.address,
                contracts.cw3_multisig.address,
                contracts.dkg.address,
                contracts.ecash.address,
                contracts.performance.address,
            ))
        }

        let nyxd = self.nyxd_details()?;

        env_content.push_str(&format!("NYXD={}\n\n", nyxd.rpc_endpoint));

        if let Ok(nym_api) = self.nym_api_endpoint() {
            env_content.push_str(&format!("NYM_API={nym_api}\n\n"));
        }

        Ok(env_content)
    }

    pub(crate) fn nyxd_details(&self) -> anyhow::Result<&NyxdDetails> {
        self.nyxd.as_ref().context("nyxd details not set")
    }

    pub(crate) fn set_nyxd_details(&mut self, account: NyxdDetails) -> &mut Self {
        self.nyxd = Some(account);
        self
    }

    pub(crate) fn localhost_rpc_endpoint(&self) -> anyhow::Result<Url> {
        let _ = self.nyxd_details()?;
        Ok("http://127.0.0.1:26657".parse()?)
    }

    /// Returns address of the nyxd rpc endpoint on the localnet container network
    #[allow(dead_code)]
    pub(crate) fn rpc_endpoint(&self) -> anyhow::Result<&Url> {
        Ok(&self.nyxd_details()?.rpc_endpoint)
    }

    pub(crate) fn nym_api_endpoint(&self) -> anyhow::Result<&Url> {
        self.nym_api_endpoint
            .as_ref()
            .context("nym api endpoint has not been set")
    }

    pub(crate) fn set_nym_api_endpoint(&mut self, nym_api_endpoint: Url) -> &mut Self {
        self.nym_api_endpoint = Some(nym_api_endpoint);
        self
    }

    pub(crate) fn auxiliary_accounts(&self) -> anyhow::Result<&AuxiliaryAccounts> {
        self.auxiliary_accounts
            .as_ref()
            .context("auxiliary accounts have not been set")
    }

    pub(crate) fn set_auxiliary_accounts(
        &mut self,
        auxiliary_accounts: AuxiliaryAccounts,
    ) -> &mut Self {
        self.auxiliary_accounts = Some(auxiliary_accounts);
        self
    }

    pub(crate) fn contracts(&self) -> anyhow::Result<&NymContracts> {
        self.contracts
            .as_ref()
            .context("cosmwasm contracts have not been initialised")
    }

    pub(crate) fn set_contracts(&mut self, contracts: NymContracts) -> &mut Self {
        self.contracts = Some(contracts);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AuxiliaryAccounts {
    pub(crate) mixnet_rewarder: Account,
    pub(crate) network_monitor: Vec<Account>,
    pub(crate) ecash_holding_account: Account,
}

impl AuxiliaryAccounts {
    pub(crate) fn new() -> Self {
        AuxiliaryAccounts {
            mixnet_rewarder: Account::new(),
            network_monitor: vec![Account::new()],
            ecash_holding_account: Account::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NyxdDetails {
    pub(crate) rpc_endpoint: Url,
    pub(crate) master_account: Account,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NymContracts {
    pub(crate) mixnet: CosmwasmContract,
    pub(crate) vesting: CosmwasmContract,
    pub(crate) ecash: CosmwasmContract,
    pub(crate) cw3_multisig: CosmwasmContract,
    pub(crate) cw4_group: CosmwasmContract,
    pub(crate) dkg: CosmwasmContract,
    pub(crate) performance: CosmwasmContract,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NymContractsBeingInitialised {
    pub(crate) mixnet: ContractBeingInitialised,
    pub(crate) vesting: ContractBeingInitialised,
    pub(crate) ecash: ContractBeingInitialised,
    pub(crate) cw3_multisig: ContractBeingInitialised,
    pub(crate) cw4_group: ContractBeingInitialised,
    pub(crate) dkg: ContractBeingInitialised,
    pub(crate) performance: ContractBeingInitialised,
}

impl NymContractsBeingInitialised {
    pub(crate) const COUNT: usize = 7;

    pub(crate) fn into_built_contracts(self) -> anyhow::Result<NymContracts> {
        Ok(NymContracts {
            mixnet: CosmwasmContract {
                address: self.mixnet.address()?.clone(),
                admin: self.mixnet.admin()?.clone(),
                name: self.mixnet.name,
            },
            vesting: CosmwasmContract {
                address: self.vesting.address()?.clone(),
                admin: self.vesting.admin()?.clone(),
                name: self.vesting.name,
            },
            ecash: CosmwasmContract {
                address: self.ecash.address()?.clone(),
                admin: self.ecash.admin()?.clone(),
                name: self.ecash.name,
            },
            cw3_multisig: CosmwasmContract {
                address: self.cw3_multisig.address()?.clone(),
                admin: self.cw3_multisig.admin()?.clone(),
                name: self.cw3_multisig.name,
            },
            cw4_group: CosmwasmContract {
                address: self.cw4_group.address()?.clone(),
                admin: self.cw4_group.admin()?.clone(),
                name: self.cw4_group.name,
            },
            dkg: CosmwasmContract {
                address: self.dkg.address()?.clone(),
                admin: self.dkg.admin()?.clone(),
                name: self.dkg.name,
            },
            performance: CosmwasmContract {
                address: self.performance.address()?.clone(),
                admin: self.performance.admin()?.clone(),
                name: self.performance.name,
            },
        })
    }

    pub(crate) fn all(&self) -> Vec<&ContractBeingInitialised> {
        vec![
            &self.mixnet,
            &self.vesting,
            &self.ecash,
            &self.cw3_multisig,
            &self.cw4_group,
            &self.dkg,
            &self.performance,
        ]
    }

    pub(crate) fn all_mut(&mut self) -> Vec<&mut ContractBeingInitialised> {
        vec![
            &mut self.mixnet,
            &mut self.vesting,
            &mut self.ecash,
            &mut self.cw3_multisig,
            &mut self.cw4_group,
            &mut self.dkg,
            &mut self.performance,
        ]
    }

    pub(crate) fn by_filename(&self, filename: &str) -> anyhow::Result<&ContractBeingInitialised> {
        if filename == contract_build_names::MIXNET {
            return Ok(&self.mixnet);
        }
        if filename == contract_build_names::VESTING {
            return Ok(&self.vesting);
        }
        if filename == contract_build_names::ECASH {
            return Ok(&self.ecash);
        }
        if filename == contract_build_names::DKG {
            return Ok(&self.dkg);
        }
        if filename == contract_build_names::GROUP {
            return Ok(&self.cw4_group);
        }
        if filename == contract_build_names::MULTISIG {
            return Ok(&self.cw3_multisig);
        }
        if filename == contract_build_names::PERFORMANCE {
            return Ok(&self.performance);
        }

        bail!("no known contract with name {filename}")
    }

    pub(crate) fn by_filename_mut(
        &mut self,
        filename: &str,
    ) -> anyhow::Result<&mut ContractBeingInitialised> {
        if filename == contract_build_names::MIXNET {
            return Ok(&mut self.mixnet);
        }
        if filename == contract_build_names::VESTING {
            return Ok(&mut self.vesting);
        }
        if filename == contract_build_names::ECASH {
            return Ok(&mut self.ecash);
        }
        if filename == contract_build_names::DKG {
            return Ok(&mut self.dkg);
        }
        if filename == contract_build_names::GROUP {
            return Ok(&mut self.cw4_group);
        }
        if filename == contract_build_names::MULTISIG {
            return Ok(&mut self.cw3_multisig);
        }
        if filename == contract_build_names::PERFORMANCE {
            return Ok(&mut self.performance);
        }

        bail!("no known contract with name {filename}")
    }

    pub(crate) fn discover_paths<P: AsRef<Path>>(&mut self, base_path: P) -> anyhow::Result<()> {
        // just look in the base path, don't traverse
        for entry_res in base_path.as_ref().read_dir()? {
            let entry = entry_res?;
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };

            if let Ok(contract) = self.by_filename_mut(&name) {
                contract.wasm_path = Some(entry.path());
            }
        }

        if let Some(no_path) = self.all().iter().find(|c| c.wasm_path.is_none()) {
            bail!(
                "could not find .wasm file for {} contract under the provided directory",
                no_path.name
            )
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_is_up_to_date() {
        let contracts = NymContractsBeingInitialised {
            mixnet: ContractBeingInitialised::new("mixnet"),
            vesting: ContractBeingInitialised::new("vesting"),
            ecash: ContractBeingInitialised::new("ecash"),
            cw3_multisig: ContractBeingInitialised::new("cw3-multisig"),
            cw4_group: ContractBeingInitialised::new("cw4-group"),
            dkg: ContractBeingInitialised::new("dkg"),
            performance: ContractBeingInitialised::new("performance"),
        };
        assert_eq!(contracts.all().len(), NymContractsBeingInitialised::COUNT);
    }

    #[test]
    fn all_and_all_mut_have_the_same_order() {
        let contracts = NymContractsBeingInitialised {
            mixnet: ContractBeingInitialised::new("mixnet"),
            vesting: ContractBeingInitialised::new("vesting"),
            ecash: ContractBeingInitialised::new("ecash"),
            cw3_multisig: ContractBeingInitialised::new("cw3-multisig"),
            cw4_group: ContractBeingInitialised::new("cw4-group"),
            dkg: ContractBeingInitialised::new("dkg"),
            performance: ContractBeingInitialised::new("performance"),
        };
        let mut contracts_clone = contracts.clone();

        for (c1, c2) in contracts.all().into_iter().zip(contracts_clone.all_mut()) {
            assert_eq!(c1, c2);
        }
    }
}
