// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::gateway::GatewayConfigUpdate;
use nym_mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use nym_mixnet_contract_common::reward_params::{IntervalRewardingParamsUpdate, Performance};
use nym_mixnet_contract_common::{
    ContractStateParams, ExecuteMsg as MixnetExecuteMsg, Gateway, Layer, LayerAssignment, MixId,
    MixNode,
};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait MixnetSigningClient {
    async fn execute_mixnet_contract(
        &self,
        fee: Option<Fee>,
        msg: MixnetExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    // state/sys-params-related

    async fn update_admin(
        &self,
        admin: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::UpdateAdmin { admin }, vec![])
            .await
    }

    async fn update_rewarding_validator_address(
        &self,
        address: AccountId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateRewardingValidatorAddress {
                address: address.to_string(),
            },
            vec![],
        )
        .await
    }

    async fn update_contract_state_params(
        &self,
        updated_parameters: ContractStateParams,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateContractStateParams { updated_parameters },
            vec![],
        )
        .await
    }

    async fn update_active_set_size(
        &self,
        active_set_size: u32,
        force_immediately: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateActiveSetSize {
                active_set_size,
                force_immediately,
            },
            vec![],
        )
        .await
    }

    async fn update_rewarding_parameters(
        &self,
        updated_params: IntervalRewardingParamsUpdate,
        force_immediately: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateRewardingParams {
                updated_params,
                force_immediately,
            },
            vec![],
        )
        .await
    }

    async fn update_interval_config(
        &self,
        epochs_in_interval: u32,
        epoch_duration_secs: u64,
        force_immediately: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateIntervalConfig {
                epochs_in_interval,
                epoch_duration_secs,
                force_immediately,
            },
            vec![],
        )
        .await
    }

    async fn begin_epoch_transition(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::BeginEpochTransition {}, vec![])
            .await
    }

    async fn advance_current_epoch(
        &self,
        new_rewarded_set: Vec<LayerAssignment>,
        expected_active_set_size: u32,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::AdvanceCurrentEpoch {
                new_rewarded_set,
                expected_active_set_size,
            },
            vec![],
        )
        .await
    }

    async fn assign_node_layer(
        &self,
        mix_id: MixId,
        layer: Layer,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::AssignNodeLayer { mix_id, layer },
            vec![],
        )
        .await
    }

    async fn reconcile_epoch_events(
        &self,
        limit: Option<u32>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::ReconcileEpochEvents { limit },
            vec![],
        )
        .await
    }

    // family related
    async fn create_family(
        &self,
        label: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::CreateFamily { label }, vec![])
            .await
    }

    async fn create_family_on_behalf(
        &self,
        owner_address: String,
        label: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::CreateFamilyOnBehalf {
                owner_address,
                label,
            },
            vec![],
        )
        .await
    }

    async fn join_family(
        &self,
        join_permit: MessageSignature,
        family_head: FamilyHead,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::JoinFamily {
                join_permit,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn join_family_on_behalf(
        &self,
        member_address: String,
        join_permit: MessageSignature,
        family_head: FamilyHead,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::JoinFamilyOnBehalf {
                member_address,
                join_permit,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn leave_family(
        &self,
        family_head: FamilyHead,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::LeaveFamily { family_head }, vec![])
            .await
    }

    async fn leave_family_on_behalf(
        &self,
        member_address: String,
        family_head: FamilyHead,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::LeaveFamilyOnBehalf {
                member_address,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn kick_family_member(
        &self,
        member: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::KickFamilyMember { member }, vec![])
            .await
    }

    async fn kick_family_member_on_behalf(
        &self,
        head_address: String,
        member: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::KickFamilyMemberOnBehalf {
                head_address,
                member,
            },
            vec![],
        )
        .await
    }
    // mixnode-related:

    async fn bond_mixnode(
        &self,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: MessageSignature,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::BondMixnode {
                mix_node,
                cost_params,
                owner_signature,
            },
            vec![pledge],
        )
        .await
    }

    async fn bond_mixnode_on_behalf(
        &self,
        owner: AccountId,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: MessageSignature,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::BondMixnodeOnBehalf {
                mix_node,
                cost_params,
                owner_signature,
                owner: owner.to_string(),
            },
            vec![pledge],
        )
        .await
    }

    async fn pledge_more(
        &self,
        additional_pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::PledgeMore {},
            vec![additional_pledge],
        )
        .await
    }

    async fn pledge_more_on_behalf(
        &self,
        owner: AccountId,
        additional_pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::PledgeMoreOnBehalf {
                owner: owner.to_string(),
            },
            vec![additional_pledge],
        )
        .await
    }

    async fn decrease_pledge(
        &self,
        decrease_by: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::DecreasePledge {
                decrease_by: decrease_by.into(),
            },
            vec![],
        )
        .await
    }

    async fn decrease_pledge_on_behalf(
        &self,
        owner: AccountId,
        decrease_by: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::DecreasePledgeOnBehalf {
                owner: owner.to_string(),
                decrease_by: decrease_by.into(),
            },
            vec![],
        )
        .await
    }

    async fn unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::UnbondMixnode {}, vec![])
            .await
    }

    async fn unbond_mixnode_on_behalf(
        &self,
        owner: AccountId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UnbondMixnodeOnBehalf {
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    async fn update_mixnode_cost_params(
        &self,
        new_costs: MixNodeCostParams,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateMixnodeCostParams { new_costs },
            vec![],
        )
        .await
    }

    async fn update_mixnode_cost_params_on_behalf(
        &self,
        owner: AccountId,
        new_costs: MixNodeCostParams,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateMixnodeCostParamsOnBehalf {
                new_costs,
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    async fn update_mixnode_config(
        &self,
        new_config: MixNodeConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateMixnodeConfig { new_config },
            vec![],
        )
        .await
    }

    async fn update_mixnode_config_on_behalf(
        &self,
        owner: AccountId,
        new_config: MixNodeConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateMixnodeConfigOnBehalf {
                new_config,
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    // gateway-related:

    async fn bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: MessageSignature,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::BondGateway {
                gateway,
                owner_signature,
            },
            vec![pledge],
        )
        .await
    }

    async fn bond_gateway_on_behalf(
        &self,
        owner: AccountId,
        gateway: Gateway,
        owner_signature: MessageSignature,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::BondGatewayOnBehalf {
                gateway,
                owner_signature,
                owner: owner.to_string(),
            },
            vec![pledge],
        )
        .await
    }

    async fn unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::UnbondGateway {}, vec![])
            .await
    }

    async fn unbond_gateway_on_behalf(
        &self,
        owner: AccountId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UnbondGatewayOnBehalf {
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    async fn update_gateway_config(
        &self,
        new_config: GatewayConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateGatewayConfig { new_config },
            vec![],
        )
        .await
    }

    async fn update_gateway_config_on_behalf(
        &self,
        owner: AccountId,
        new_config: GatewayConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UpdateGatewayConfigOnBehalf {
                new_config,
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    // delegation-related:

    async fn delegate_to_mixnode(
        &self,
        mix_id: MixId,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::DelegateToMixnode { mix_id },
            vec![amount],
        )
        .await
    }

    async fn delegate_to_mixnode_on_behalf(
        &self,
        delegate: AccountId,
        mix_id: MixId,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::DelegateToMixnodeOnBehalf {
                mix_id,
                delegate: delegate.to_string(),
            },
            vec![amount],
        )
        .await
    }

    async fn undelegate_from_mixnode(
        &self,
        mix_id: MixId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UndelegateFromMixnode { mix_id },
            vec![],
        )
        .await
    }

    async fn undelegate_to_mixnode_on_behalf(
        &self,
        delegate: AccountId,
        mix_id: MixId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::UndelegateFromMixnodeOnBehalf {
                mix_id,
                delegate: delegate.to_string(),
            },
            vec![],
        )
        .await
    }

    // reward-related

    async fn reward_mixnode(
        &self,
        mix_id: MixId,
        performance: Performance,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::RewardMixnode {
                mix_id,
                performance,
            },
            vec![],
        )
        .await
    }

    async fn withdraw_operator_reward(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::WithdrawOperatorReward {}, vec![])
            .await
    }

    async fn withdraw_operator_reward_on_behalf(
        &self,
        owner: AccountId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::WithdrawOperatorRewardOnBehalf {
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    async fn withdraw_delegator_reward(
        &self,
        mix_id: MixId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::WithdrawDelegatorReward { mix_id },
            vec![],
        )
        .await
    }

    async fn withdraw_delegator_reward_on_behalf(
        &self,
        owner: AccountId,
        mix_id: MixId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::WithdrawDelegatorRewardOnBehalf {
                mix_id,
                owner: owner.to_string(),
            },
            vec![],
        )
        .await
    }

    async fn migrate_vested_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(fee, MixnetExecuteMsg::MigrateVestedMixNode {}, vec![])
            .await
    }

    async fn migrate_vested_delegation(
        &self,
        mix_id: MixId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::MigrateVestedDelegation { mix_id },
            vec![],
        )
        .await
    }

    #[cfg(feature = "contract-testing")]
    async fn testing_resolve_all_pending_events(
        &self,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::TestingResolveAllPendingEvents { limit: None },
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> MixnetSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_mixnet_contract(
        &self,
        fee: Option<Fee>,
        msg: MixnetExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let mixnet_contract_address = &self
            .mixnet_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("mixnet contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let memo = msg.default_memo();

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            mixnet_contract_address,
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
    use crate::nyxd::contract_traits::tests::{mock_coin, IgnoreValue};
    use nym_mixnet_contract_common::ExecuteMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: MixnetSigningClient + Send + Sync>(
        client: C,
        msg: MixnetExecuteMsg,
    ) {
        match msg {
            ExecuteMsg::UpdateAdmin { admin } => client.update_admin(admin, None).ignore(),
            MixnetExecuteMsg::AssignNodeLayer { mix_id, layer } => {
                client.assign_node_layer(mix_id, layer, None).ignore()
            }
            MixnetExecuteMsg::CreateFamily { label } => client.create_family(label, None).ignore(),
            MixnetExecuteMsg::JoinFamily {
                join_permit,
                family_head,
            } => client.join_family(join_permit, family_head, None).ignore(),
            MixnetExecuteMsg::LeaveFamily { family_head } => {
                client.leave_family(family_head, None).ignore()
            }
            MixnetExecuteMsg::KickFamilyMember { member } => {
                client.kick_family_member(member, None).ignore()
            }
            MixnetExecuteMsg::CreateFamilyOnBehalf {
                owner_address,
                label,
            } => client
                .create_family_on_behalf(owner_address, label, None)
                .ignore(),
            MixnetExecuteMsg::JoinFamilyOnBehalf {
                member_address,
                join_permit,
                family_head,
            } => client
                .join_family_on_behalf(member_address, join_permit, family_head, None)
                .ignore(),
            MixnetExecuteMsg::LeaveFamilyOnBehalf {
                member_address,
                family_head,
            } => client
                .leave_family_on_behalf(member_address, family_head, None)
                .ignore(),
            MixnetExecuteMsg::KickFamilyMemberOnBehalf {
                head_address,
                member,
            } => client
                .kick_family_member_on_behalf(head_address, member, None)
                .ignore(),
            MixnetExecuteMsg::UpdateRewardingValidatorAddress { address } => client
                .update_rewarding_validator_address(address.parse().unwrap(), None)
                .ignore(),
            MixnetExecuteMsg::UpdateContractStateParams { updated_parameters } => client
                .update_contract_state_params(updated_parameters, None)
                .ignore(),
            MixnetExecuteMsg::UpdateActiveSetSize {
                active_set_size,
                force_immediately,
            } => client
                .update_active_set_size(active_set_size, force_immediately, None)
                .ignore(),
            MixnetExecuteMsg::UpdateRewardingParams {
                updated_params,
                force_immediately,
            } => client
                .update_rewarding_parameters(updated_params, force_immediately, None)
                .ignore(),
            MixnetExecuteMsg::UpdateIntervalConfig {
                epochs_in_interval,
                epoch_duration_secs,
                force_immediately,
            } => client
                .update_interval_config(
                    epochs_in_interval,
                    epoch_duration_secs,
                    force_immediately,
                    None,
                )
                .ignore(),
            MixnetExecuteMsg::BeginEpochTransition {} => {
                client.begin_epoch_transition(None).ignore()
            }
            MixnetExecuteMsg::AdvanceCurrentEpoch {
                new_rewarded_set,
                expected_active_set_size,
            } => client
                .advance_current_epoch(new_rewarded_set, expected_active_set_size, None)
                .ignore(),
            MixnetExecuteMsg::ReconcileEpochEvents { limit } => {
                client.reconcile_epoch_events(limit, None).ignore()
            }
            MixnetExecuteMsg::BondMixnode {
                mix_node,
                cost_params,
                owner_signature,
            } => client
                .bond_mixnode(mix_node, cost_params, owner_signature, mock_coin(), None)
                .ignore(),
            MixnetExecuteMsg::BondMixnodeOnBehalf {
                mix_node,
                cost_params,
                owner_signature,
                owner,
            } => client
                .bond_mixnode_on_behalf(
                    owner.parse().unwrap(),
                    mix_node,
                    cost_params,
                    owner_signature,
                    mock_coin(),
                    None,
                )
                .ignore(),
            MixnetExecuteMsg::PledgeMore {} => client.pledge_more(mock_coin(), None).ignore(),
            MixnetExecuteMsg::PledgeMoreOnBehalf { owner } => client
                .pledge_more_on_behalf(owner.parse().unwrap(), mock_coin(), None)
                .ignore(),
            MixnetExecuteMsg::DecreasePledge { decrease_by } => {
                client.decrease_pledge(decrease_by.into(), None).ignore()
            }
            MixnetExecuteMsg::DecreasePledgeOnBehalf { owner, decrease_by } => client
                .decrease_pledge_on_behalf(owner.parse().unwrap(), decrease_by.into(), None)
                .ignore(),
            MixnetExecuteMsg::UnbondMixnode {} => client.unbond_mixnode(None).ignore(),
            MixnetExecuteMsg::UnbondMixnodeOnBehalf { owner } => client
                .unbond_mixnode_on_behalf(owner.parse().unwrap(), None)
                .ignore(),
            MixnetExecuteMsg::UpdateMixnodeCostParams { new_costs } => {
                client.update_mixnode_cost_params(new_costs, None).ignore()
            }
            MixnetExecuteMsg::UpdateMixnodeCostParamsOnBehalf { new_costs, owner } => client
                .update_mixnode_cost_params_on_behalf(owner.parse().unwrap(), new_costs, None)
                .ignore(),
            MixnetExecuteMsg::UpdateMixnodeConfig { new_config } => {
                client.update_mixnode_config(new_config, None).ignore()
            }
            MixnetExecuteMsg::UpdateMixnodeConfigOnBehalf { new_config, owner } => client
                .update_mixnode_config_on_behalf(owner.parse().unwrap(), new_config, None)
                .ignore(),
            MixnetExecuteMsg::BondGateway {
                gateway,
                owner_signature,
            } => client
                .bond_gateway(gateway, owner_signature, mock_coin(), None)
                .ignore(),
            MixnetExecuteMsg::BondGatewayOnBehalf {
                gateway,
                owner,
                owner_signature,
            } => client
                .bond_gateway_on_behalf(
                    owner.parse().unwrap(),
                    gateway,
                    owner_signature,
                    mock_coin(),
                    None,
                )
                .ignore(),
            MixnetExecuteMsg::UnbondGateway {} => client.unbond_gateway(None).ignore(),
            MixnetExecuteMsg::UnbondGatewayOnBehalf { owner } => client
                .unbond_gateway_on_behalf(owner.parse().unwrap(), None)
                .ignore(),
            MixnetExecuteMsg::UpdateGatewayConfig { new_config } => {
                client.update_gateway_config(new_config, None).ignore()
            }
            MixnetExecuteMsg::UpdateGatewayConfigOnBehalf { new_config, owner } => client
                .update_gateway_config_on_behalf(owner.parse().unwrap(), new_config, None)
                .ignore(),
            MixnetExecuteMsg::DelegateToMixnode { mix_id } => client
                .delegate_to_mixnode(mix_id, mock_coin(), None)
                .ignore(),
            MixnetExecuteMsg::DelegateToMixnodeOnBehalf { mix_id, delegate } => client
                .delegate_to_mixnode_on_behalf(delegate.parse().unwrap(), mix_id, mock_coin(), None)
                .ignore(),
            MixnetExecuteMsg::UndelegateFromMixnode { mix_id } => {
                client.undelegate_from_mixnode(mix_id, None).ignore()
            }
            MixnetExecuteMsg::UndelegateFromMixnodeOnBehalf { mix_id, delegate } => client
                .undelegate_to_mixnode_on_behalf(delegate.parse().unwrap(), mix_id, None)
                .ignore(),
            MixnetExecuteMsg::RewardMixnode {
                mix_id,
                performance,
            } => client.reward_mixnode(mix_id, performance, None).ignore(),
            MixnetExecuteMsg::WithdrawOperatorReward {} => {
                client.withdraw_operator_reward(None).ignore()
            }
            MixnetExecuteMsg::WithdrawOperatorRewardOnBehalf { owner } => client
                .withdraw_operator_reward_on_behalf(owner.parse().unwrap(), None)
                .ignore(),
            MixnetExecuteMsg::WithdrawDelegatorReward { mix_id } => {
                client.withdraw_delegator_reward(mix_id, None).ignore()
            }
            MixnetExecuteMsg::WithdrawDelegatorRewardOnBehalf { mix_id, owner } => client
                .withdraw_delegator_reward_on_behalf(owner.parse().unwrap(), mix_id, None)
                .ignore(),
            MixnetExecuteMsg::MigrateVestedMixNode { .. } => {
                client.migrate_vested_mixnode(None).ignore()
            }
            MixnetExecuteMsg::MigrateVestedDelegation { mix_id } => {
                client.migrate_vested_delegation(mix_id, None).ignore()
            }

            #[cfg(feature = "contract-testing")]
            MixnetExecuteMsg::TestingResolveAllPendingEvents { .. } => {
                client.testing_resolve_all_pending_events(None).ignore()
            }
        };
    }
}
