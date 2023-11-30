// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::{BandwidthInfo, ContractsInfo, DkgInfo, GroupInfo, MultisigInfo};
use crate::cli::Args;
use crate::components::basic_contract_info::BasicContractInfo;
use crate::utils::zero_coin;
use futures::future::{join, join5};
use nym_coconut_dkg_common::types::Addr;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, DkgSigningClient, GroupQueryClient, GroupSigningClient, NymContractsProvider,
    PagedDkgQueryClient, PagedGroupQueryClient,
};
use nym_validator_client::nyxd::cw4::Cw4Contract;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{cw2, cw4, AccountId, CosmWasmClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

#[derive(Clone)]
pub struct NyxdClient(Arc<RwLock<Inner>>);

struct Inner(DirectSigningHttpRpcNyxdClient);

impl Deref for Inner {
    type Target = DirectSigningHttpRpcNyxdClient;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub struct DkgState {
    pub mix_denom: String,
    pub multisig_addr: Addr,
    pub group_addr: Cw4Contract,
}

impl Inner {
    // that's nasty, but it works
    pub async fn get_dkg_state(&self) -> anyhow::Result<DkgState> {
        let dkg_contract_address = &self
            .dkg_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("dkg contract"))?;

        let res = self
            .query_contract_raw(dkg_contract_address, b"state".to_vec())
            .await?;
        Ok(serde_json::from_slice(&res).map_err(NyxdError::from)?)
    }

    pub async fn get_raw_cw2_version(
        &self,
        contract: &AccountId,
    ) -> anyhow::Result<cw2::ContractVersion> {
        let res = self
            .query_contract_raw(contract, b"contract_info".to_vec())
            .await?;
        Ok(serde_json::from_slice(&res).map_err(NyxdError::from)?)
    }
}

impl NyxdClient {
    pub async fn dkg_contract(&self) -> anyhow::Result<AccountId> {
        Ok(self
            .0
            .read()
            .await
            .dkg_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("dkg contract"))?
            .clone())
    }

    pub async fn group_contract(&self) -> anyhow::Result<AccountId> {
        Ok(self
            .0
            .read()
            .await
            .group_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("group contract"))?
            .clone())
    }

    pub async fn bandwidth_contract(&self) -> anyhow::Result<AccountId> {
        Ok(self
            .0
            .read()
            .await
            .coconut_bandwidth_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("bandwidth contract"))?
            .clone())
    }

    pub async fn multisig_contract(&self) -> anyhow::Result<AccountId> {
        Ok(self
            .0
            .read()
            .await
            .multisig_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("multisig contract"))?
            .clone())
    }

    pub async fn address(&self) -> AccountId {
        self.0.read().await.address()
    }

    pub async fn dkg_update(&self) -> anyhow::Result<DkgInfo> {
        let address = self.dkg_contract().await?;
        let guard = self.0.read().await;

        let balance_fut = guard.get_balance(&address, "unym".to_string());

        let epoch_fut = guard.get_current_epoch();
        let threshold_fut = guard.get_current_epoch_threshold();
        let dealers_fut = guard.get_all_current_dealers();
        let past_dealers_fut = guard.get_all_past_dealers();
        let state_fut = guard.get_dkg_state();

        let info_res = join5(
            epoch_fut,
            threshold_fut,
            dealers_fut,
            past_dealers_fut,
            state_fut,
        )
        .await;

        let dkg_epoch = info_res.0?;

        let epoch_dealings_fut = guard.get_all_epoch_dealings(dkg_epoch.epoch_id);
        let epoch_vk_shares_fut = guard.get_all_verification_key_shares(dkg_epoch.epoch_id);

        let epoch_res = join(epoch_dealings_fut, epoch_vk_shares_fut).await;

        Ok(DkgInfo {
            base: BasicContractInfo {
                name: "DKG Contract".to_string(),
                address: address.to_string(),
                balance: balance_fut.await?.unwrap_or(zero_coin()),
                cw2_version: None,
                build_info: None,
            },
            epoch: dkg_epoch,
            threshold: info_res.1?,
            dealers: info_res.2?,
            past_dealers: info_res.3?,
            debug_state: info_res.4?,
            epoch_dealings: epoch_res.0?,
            vk_shares: epoch_res.1?,
        })
    }

    pub async fn group_update(&self) -> anyhow::Result<GroupInfo> {
        let address = self.group_contract().await?;
        let guard = self.0.read().await;

        let balance_fut = guard.get_balance(&address, "unym".to_string());
        let cw_version = guard.get_raw_cw2_version(&address);

        let admin_fut = guard.admin();
        let member_fut = guard.get_all_members();
        let total_fut = guard.total_weight(None);

        let res = join5(balance_fut, cw_version, admin_fut, member_fut, total_fut).await;

        Ok(GroupInfo {
            base: BasicContractInfo {
                name: "CW4 Group Contract".to_string(),
                address: address.to_string(),
                balance: res.0?.unwrap_or(zero_coin()),
                cw2_version: Some(res.1?),
                build_info: None,
            },
            admin: res.2?,
            members: res.3?,
            total_weight: res.4?,
        })
    }

    pub async fn bandwidth_update(&self) -> anyhow::Result<BandwidthInfo> {
        let address = self.bandwidth_contract().await?;
        let guard = self.0.read().await;

        let balance_fut = guard.get_balance(&address, "unym".to_string());

        Ok(BandwidthInfo {
            base: BasicContractInfo {
                name: "Coconut Bandwidth Contract".to_string(),
                address: address.to_string(),
                balance: balance_fut.await?.unwrap_or(zero_coin()),
                cw2_version: None,
                build_info: None,
            },
        })
    }

    pub async fn multisig_update(&self) -> anyhow::Result<MultisigInfo> {
        let address = self.multisig_contract().await?;
        let guard = self.0.read().await;

        let balance_fut = guard.get_balance(&address, "unym".to_string());
        let cw_version = guard.get_raw_cw2_version(&address);

        let res = join(balance_fut, cw_version).await;

        Ok(MultisigInfo {
            base: BasicContractInfo {
                name: "CW3 Flex Multisig".to_string(),
                address: address.to_string(),
                balance: res.0?.unwrap_or(zero_coin()),
                cw2_version: Some(res.1?),
                build_info: None,
            },
        })
    }

    pub async fn get_contract_update(&self) -> anyhow::Result<ContractsInfo> {
        Ok(ContractsInfo {
            dkg: self.dkg_update().await?,
            group: self.group_update().await?,
            bandwidth: self.bandwidth_update().await?,
            multisig: self.multisig_update().await?,
        })
    }

    pub async fn add_group_member(&self, addr: String, weight: u64) -> anyhow::Result<()> {
        let member = cw4::Member { addr, weight };

        // we need to have a write lock here so that we wouldn't accidentally send multiple transactions
        // into the same block (and thus have invalid seq numbers)
        self.0
            .write()
            .await
            .update_members(vec![member], vec![], None)
            .await?;
        Ok(())
    }

    pub async fn remove_group_member(&self, address: String) -> anyhow::Result<()> {
        // we need to have a write lock here so that we wouldn't accidentally send multiple transactions
        // into the same block (and thus have invalid seq numbers)
        self.0
            .write()
            .await
            .update_members(vec![], vec![address], None)
            .await?;
        Ok(())
    }

    pub async fn try_advance_epoch_state(&self) -> anyhow::Result<()> {
        // we need to have a write lock here so that we wouldn't accidentally send multiple transactions
        // into the same block (and thus have invalid seq numbers)
        self.0.write().await.advance_dkg_epoch_state(None).await?;
        Ok(())
    }

    pub async fn try_surpass_threshold(&self) -> anyhow::Result<()> {
        // we need to have a write lock here so that we wouldn't accidentally send multiple transactions
        // into the same block (and thus have invalid seq numbers)
        self.0.write().await.surpass_threshold(None).await?;
        Ok(())
    }
}

pub fn setup_nyxd_client(args: Args) -> anyhow::Result<(NyxdClient, Url)> {
    let mut network_details = NymNetworkDetails::new_from_env();

    if let Some(dkg_contract) = args.dkg_contract_address {
        network_details.contracts.coconut_dkg_contract_address = Some(dkg_contract.to_string());
    }

    let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

    let validator_endpoint = match args.nyxd_validator {
        Some(endpoint) => endpoint,
        None => network_details.endpoints[0].nyxd_url(),
    };

    Ok((
        NyxdClient(Arc::new(RwLock::new(Inner(
            DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
                client_config,
                validator_endpoint.as_ref(),
                args.admin_mnemonic,
            )?,
        )))),
        validator_endpoint,
    ))
}
