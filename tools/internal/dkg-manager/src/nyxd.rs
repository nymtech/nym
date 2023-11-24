// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::ContractsInfo;
use crate::cli::Args;
use futures::future::{join3, join5};
use nym_coconut_dkg_common::types::Addr;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, GroupQueryClient, GroupSigningClient, NymContractsProvider,
    PagedDkgQueryClient, PagedGroupQueryClient,
};
use nym_validator_client::nyxd::cw4::Cw4Contract;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{cw4, AccountId, CosmWasmClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;

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

    pub async fn address(&self) -> AccountId {
        self.0.read().await.address()
    }

    pub async fn get_dkg_update(&self) -> anyhow::Result<ContractsInfo> {
        let guard = self.0.read().await;

        let epoch_fut = guard.get_current_epoch();
        let threshold_fut = guard.get_current_epoch_threshold();
        let dealers_fut = guard.get_all_current_dealers();
        let past_dealers_fut = guard.get_all_past_dealers();
        let state_fut = guard.get_dkg_state();

        let dkg_futs = join5(
            epoch_fut,
            threshold_fut,
            dealers_fut,
            past_dealers_fut,
            state_fut,
        )
        .await;

        let group_admin_fut = guard.admin();
        let group_member_fut = guard.get_all_members();
        let group_total_fut = guard.total_weight(None);

        let group_futs = join3(group_admin_fut, group_member_fut, group_total_fut).await;

        Ok(ContractsInfo {
            dkg_epoch: dkg_futs.0?,
            threshold: dkg_futs.1?,
            dealers: dkg_futs.2?,
            past_dealers: dkg_futs.3?,
            dkg_state: dkg_futs.4?,
            group_admin: group_futs.0?,
            group_members: group_futs.1?,
            total_weight: group_futs.2?,
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
}

pub fn setup_nyxd_client(args: Args) -> anyhow::Result<NyxdClient> {
    let mut network_details = NymNetworkDetails::new_from_env();

    if let Some(dkg_contract) = args.dkg_contract_address {
        network_details.contracts.coconut_dkg_contract_address = Some(dkg_contract.to_string());
    }

    let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

    let validator_endpoint = match args.nyxd_validator {
        Some(endpoint) => endpoint,
        None => network_details.endpoints[0].nyxd_url(),
    };

    Ok(NyxdClient(Arc::new(RwLock::new(Inner(
        DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            validator_endpoint.as_ref(),
            args.admin_mnemonic,
        )?,
    )))))
}
