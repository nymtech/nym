// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::action::ContractsInfo;
use crate::cli::Args;
use futures::future::join3;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, GroupQueryClient, GroupSigningClient, PagedDkgQueryClient,
    PagedGroupQueryClient,
};
use nym_validator_client::nyxd::cw4;
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct NyxdClient(pub Arc<RwLock<DirectSigningHttpRpcNyxdClient>>);

impl NyxdClient {
    pub async fn get_dkg_update(&self) -> anyhow::Result<ContractsInfo> {
        let guard = self.0.read().await;

        let epoch_fut = guard.get_current_epoch();
        let threshold_fut = guard.get_current_epoch_threshold();
        let dealers_fut = guard.get_all_current_dealers();

        let dkg_futs = join3(epoch_fut, threshold_fut, dealers_fut).await;

        let group_admin_fut = guard.admin();
        let group_member_fut = guard.get_all_members();
        let group_total_fut = guard.total_weight(None);

        let group_futs = join3(group_admin_fut, group_member_fut, group_total_fut).await;

        Ok(ContractsInfo {
            dkg_epoch: dkg_futs.0?,
            threshold: dkg_futs.1?,
            dealers: dkg_futs.2?,
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

    Ok(NyxdClient(Arc::new(RwLock::new(
        DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            validator_endpoint.as_ref(),
            args.admin_mnemonic,
        )?,
    ))))
}
