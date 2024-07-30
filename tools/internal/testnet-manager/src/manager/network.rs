// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NetworkManagerError;
use crate::helpers::default_storage_dir;
use crate::manager::contract::{Account, LoadedNymContracts, NymContracts};
use nym_config::defaults::{NymNetworkDetails, ValidatorDetails};
use nym_validator_client::nyxd::Config;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use time::OffsetDateTime;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Network {
    pub name: String,

    pub rpc_endpoint: Url,

    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,

    pub contracts: NymContracts,

    pub auxiliary_addresses: SpecialAddresses,
}

impl Network {
    pub fn into_loaded(self) -> LoadedNetwork {
        self.into()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct LoadedNetwork {
    pub(crate) id: i64,
    pub(crate) name: String,

    pub(crate) rpc_endpoint: Url,

    #[serde(with = "time::serde::rfc3339")]
    pub(crate) created_at: OffsetDateTime,

    pub(crate) contracts: LoadedNymContracts,

    pub(crate) auxiliary_addresses: SpecialAddresses,
}

impl From<Network> for LoadedNetwork {
    fn from(value: Network) -> Self {
        LoadedNetwork {
            id: i64::MAX,
            name: value.name,
            rpc_endpoint: value.rpc_endpoint,
            created_at: value.created_at,
            contracts: value.contracts.into(),
            auxiliary_addresses: value.auxiliary_addresses,
        }
    }
}

impl<'a> From<&'a LoadedNetwork> for nym_config::defaults::NymNetworkDetails {
    fn from(value: &'a LoadedNetwork) -> Self {
        let contracts = nym_config::defaults::NymContracts {
            mixnet_contract_address: Some(value.contracts.mixnet.address.to_string()),
            vesting_contract_address: Some(value.contracts.vesting.address.to_string()),
            ecash_contract_address: Some(value.contracts.ecash.address.to_string()),
            group_contract_address: Some(value.contracts.cw4_group.address.to_string()),
            multisig_contract_address: Some(value.contracts.cw3_multisig.address.to_string()),
            coconut_dkg_contract_address: Some(value.contracts.dkg.address.to_string()),
        };
        // ASSUMPTION: same chain details like prefix, denoms, etc. as mainnet
        let mainnet = NymNetworkDetails::new_mainnet();
        NymNetworkDetails {
            chain_details: mainnet.chain_details,
            network_name: "foomp".to_string(),
            endpoints: vec![ValidatorDetails {
                nyxd_url: value.rpc_endpoint.to_string(),
                websocket_url: None,
                api_url: None,
            }],
            contracts,
            explorer_api: None,
            nym_vpn_api_url: None,
        }
    }
}

impl LoadedNetwork {
    pub fn default_env_file_path(&self) -> PathBuf {
        default_storage_dir()
            .join(&self.name)
            .join(format!("{}.env", &self.name))
    }

    #[allow(dead_code)]
    pub fn query_client(&self) -> Result<QueryHttpRpcNyxdClient, NetworkManagerError> {
        Ok(QueryHttpRpcNyxdClient::connect(
            self.client_config()?,
            self.rpc_endpoint.as_str(),
        )?)
    }

    pub fn dkg_signing_client(
        &self,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            self.client_config()?,
            self.rpc_endpoint.as_str(),
            self.contracts.dkg.admin_mnemonic.clone(),
        )?)
    }

    pub fn client_config(&self) -> Result<Config, NetworkManagerError> {
        let network_details = NymNetworkDetails::from(self);
        let config = Config::try_from_nym_network_details(&network_details)?;
        Ok(config)
    }

    pub fn cw4_group_signing_client(
        &self,
    ) -> Result<DirectSigningHttpRpcNyxdClient, NetworkManagerError> {
        Ok(DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            self.client_config()?,
            self.rpc_endpoint.as_str(),
            self.contracts.cw4_group.admin_mnemonic.clone(),
        )?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialAddresses {
    pub ecash_holding_account: Account,
    pub mixnet_rewarder: Account,
}

impl Default for SpecialAddresses {
    fn default() -> Self {
        SpecialAddresses {
            ecash_holding_account: Account::new(),
            mixnet_rewarder: Account::new(),
        }
    }
}
