// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use node_families_contract_common::{Config, ExecuteMsg as NodeFamiliesExecuteMsg, NodeFamilyId};
use nym_mixnet_contract_common::NodeId;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NodeFamiliesSigningClient {
    async fn execute_node_families_contract(
        &self,
        fee: Option<Fee>,
        msg: NodeFamiliesExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn update_node_families_config(
        &self,
        config: Config,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::UpdateConfig { config },
            "NodeFamiliesContract::UpdateConfig".to_string(),
            vec![],
        )
        .await
    }

    async fn create_family(
        &self,
        name: String,
        description: String,
        fee: Option<Fee>,
        creation_fee: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::CreateFamily { name, description },
            "NodeFamiliesContract::CreateFamily".to_string(),
            creation_fee,
        )
        .await
    }

    async fn disband_family(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::DisbandFamily {},
            "NodeFamiliesContract::DisbandFamily".to_string(),
            vec![],
        )
        .await
    }

    async fn invite_to_family(
        &self,
        node_id: NodeId,
        validity_secs: Option<u64>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::InviteToFamily {
                node_id,
                validity_secs,
            },
            "NodeFamiliesContract::InviteToFamily".to_string(),
            vec![],
        )
        .await
    }

    async fn revoke_family_invitation(
        &self,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::RevokeFamilyInvitation { node_id },
            "NodeFamiliesContract::RevokeFamilyInvitation".to_string(),
            vec![],
        )
        .await
    }

    async fn accept_family_invitation(
        &self,
        family_id: NodeFamilyId,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::AcceptFamilyInvitation { family_id, node_id },
            "NodeFamiliesContract::AcceptFamilyInvitation".to_string(),
            vec![],
        )
        .await
    }

    async fn reject_family_invitation(
        &self,
        family_id: NodeFamilyId,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::RejectFamilyInvitation { family_id, node_id },
            "NodeFamiliesContract::RejectFamilyInvitation".to_string(),
            vec![],
        )
        .await
    }

    async fn leave_family(
        &self,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::LeaveFamily { node_id },
            "NodeFamiliesContract::LeaveFamily".to_string(),
            vec![],
        )
        .await
    }

    async fn kick_from_family(
        &self,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::KickFromFamily { node_id },
            "NodeFamiliesContract::KickFromFamily".to_string(),
            vec![],
        )
        .await
    }

    /// Cross-contract callback fired by the mixnet contract on node unbonding.
    /// Exposed for completeness; the families contract rejects this call from
    /// any sender other than the configured mixnet contract address.
    async fn on_nym_node_unbond(
        &self,
        node_id: NodeId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_node_families_contract(
            fee,
            NodeFamiliesExecuteMsg::OnNymNodeUnbond { node_id },
            "NodeFamiliesContract::OnNymNodeUnbond".to_string(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> NodeFamiliesSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_node_families_contract(
        &self,
        fee: Option<Fee>,
        msg: NodeFamiliesExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let node_families_contract_address = &self
            .node_families_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("node families contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()[0];
        self.execute(
            signer_address,
            node_families_contract_address,
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
    use node_families_contract_common::ExecuteMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: NodeFamiliesSigningClient + Send + Sync>(
        client: C,
        msg: NodeFamiliesExecuteMsg,
    ) {
        match msg {
            NodeFamiliesExecuteMsg::UpdateConfig { config } => {
                client.update_node_families_config(config, None).ignore()
            }
            NodeFamiliesExecuteMsg::CreateFamily { name, description } => client
                .create_family(name, description, None, vec![])
                .ignore(),
            NodeFamiliesExecuteMsg::DisbandFamily {} => client.disband_family(None).ignore(),
            NodeFamiliesExecuteMsg::InviteToFamily {
                node_id,
                validity_secs,
            } => client
                .invite_to_family(node_id, validity_secs, None)
                .ignore(),
            NodeFamiliesExecuteMsg::RevokeFamilyInvitation { node_id } => {
                client.revoke_family_invitation(node_id, None).ignore()
            }
            NodeFamiliesExecuteMsg::AcceptFamilyInvitation { family_id, node_id } => client
                .accept_family_invitation(family_id, node_id, None)
                .ignore(),
            NodeFamiliesExecuteMsg::RejectFamilyInvitation { family_id, node_id } => client
                .reject_family_invitation(family_id, node_id, None)
                .ignore(),
            NodeFamiliesExecuteMsg::LeaveFamily { node_id } => {
                client.leave_family(node_id, None).ignore()
            }
            NodeFamiliesExecuteMsg::KickFromFamily { node_id } => {
                client.kick_from_family(node_id, None).ignore()
            }
            ExecuteMsg::OnNymNodeUnbond { node_id } => {
                client.on_nym_node_unbond(node_id, None).ignore()
            }
        };
    }
}
