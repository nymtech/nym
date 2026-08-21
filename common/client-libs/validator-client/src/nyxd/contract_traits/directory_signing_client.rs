// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cosmwasm_std::Binary;
use nym_directory_contract_common::ExecuteMsg as DirectoryExecuteMsg;
use nym_mixnet_contract_common::NodeId;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DirectorySigningClient {
    async fn execute_directory_contract(
        &self,
        fee: Option<Fee>,
        msg: DirectoryExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn set_node_entry(
        &self,
        node_id: NodeId,
        label: String,
        data: Binary,
        sequence: u64,
        signature: Binary,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::SetNodeEntry {
                node_id,
                label,
                data,
                sequence,
                signature,
            },
            "DirectoryContract::SetNodeEntry".to_string(),
            vec![],
        )
        .await
    }

    async fn delete_node_entry(
        &self,
        node_id: NodeId,
        label: String,
        sequence: u64,
        signature: Binary,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::DeleteNodeEntry {
                node_id,
                label,
                sequence,
                signature,
            },
            "DirectoryContract::DeleteNodeEntry".to_string(),
            vec![],
        )
        .await
    }

    async fn set_curated_entry(
        &self,
        key: String,
        data: Binary,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::SetCuratedEntry { key, data },
            "DirectoryContract::SetCuratedEntry".to_string(),
            vec![],
        )
        .await
    }

    async fn remove_curated_entry(
        &self,
        key: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::RemoveCuratedEntry { key },
            "DirectoryContract::RemoveCuratedEntry".to_string(),
            vec![],
        )
        .await
    }

    async fn set_label(
        &self,
        label: String,
        max_size: u32,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::SetLabel { label, max_size },
            "DirectoryContract::SetLabel".to_string(),
            vec![],
        )
        .await
    }

    async fn remove_label(
        &self,
        label: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::RemoveLabel { label },
            "DirectoryContract::RemoveLabel".to_string(),
            vec![],
        )
        .await
    }

    async fn update_admin(
        &self,
        admin: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::UpdateAdmin { admin },
            "DirectoryContract::UpdateAdmin".to_string(),
            vec![],
        )
        .await
    }

    /// Cross-contract callback fired by the mixnet contract on node unbonding.
    /// Exposed for completeness; the directory contract rejects this call from
    /// any sender other than the configured mixnet contract address.
    async fn on_nym_node_unbond(
        &self,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::OnNymNodeUnbond { node_id },
            "DirectoryContract::OnNymNodeUnbond".to_string(),
            vec![],
        )
        .await
    }

    async fn update_snapshot_interval(
        &self,
        interval: u32,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_directory_contract(
            fee,
            DirectoryExecuteMsg::UpdateSnapshotInterval { interval },
            "DirectoryContract::UpdateSnapshotInterval".to_string(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> DirectorySigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_directory_contract(
        &self,
        fee: Option<Fee>,
        msg: DirectoryExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let directory_contract_address = &self
            .directory_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("directory contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()[0];
        self.execute(
            signer_address,
            directory_contract_address,
            &msg,
            fee,
            memo,
            funds,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: DirectorySigningClient + Send + Sync>(
        client: C,
        msg: DirectoryExecuteMsg,
    ) {
        match msg {
            DirectoryExecuteMsg::SetNodeEntry {
                node_id,
                label,
                data,
                sequence,
                signature,
            } => client
                .set_node_entry(node_id, label, data, sequence, signature, None)
                .ignore(),
            DirectoryExecuteMsg::DeleteNodeEntry {
                node_id,
                label,
                sequence,
                signature,
            } => client
                .delete_node_entry(node_id, label, sequence, signature, None)
                .ignore(),
            DirectoryExecuteMsg::SetCuratedEntry { key, data } => {
                client.set_curated_entry(key, data, None).ignore()
            }
            DirectoryExecuteMsg::RemoveCuratedEntry { key } => {
                client.remove_curated_entry(key, None).ignore()
            }
            DirectoryExecuteMsg::SetLabel { label, max_size } => {
                client.set_label(label, max_size, None).ignore()
            }
            DirectoryExecuteMsg::RemoveLabel { label } => client.remove_label(label, None).ignore(),
            DirectoryExecuteMsg::UpdateAdmin { admin } => client.update_admin(admin, None).ignore(),
            DirectoryExecuteMsg::OnNymNodeUnbond { node_id } => {
                client.on_nym_node_unbond(node_id, None).ignore()
            }
            DirectoryExecuteMsg::UpdateSnapshotInterval { interval } => {
                client.update_snapshot_interval(interval, None).ignore()
            }
        };
    }
}
