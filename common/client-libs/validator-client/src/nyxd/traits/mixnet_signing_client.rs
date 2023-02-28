// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::coin::Coin;
pub use crate::nyxd::cosmwasm_client::client::CosmWasmClient;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, NyxdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use nym_mixnet_contract_common::reward_params::{IntervalRewardingParamsUpdate, Performance};
use nym_mixnet_contract_common::{
    ContractStateParams, ExecuteMsg as MixnetExecuteMsg, Gateway, LayerAssignment, MixId, MixNode,
};

#[async_trait]
pub trait MixnetSigningClient {
    async fn execute_mixnet_contract(
        &self,
        fee: Option<Fee>,
        msg: MixnetExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    // state/sys-params-related

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
        owner_signature: String,
        label: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::CreateFamily {
                owner_signature,
                label,
            },
            vec![],
        )
        .await
    }

    async fn create_family_on_behalf(
        &self,
        owner_address: String,
        owner_signature: String,
        label: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::CreateFamilyOnBehalf {
                owner_address,
                owner_signature,
                label,
            },
            vec![],
        )
        .await
    }

    async fn join_family(
        &self,
        signature: String,
        family_head: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::JoinFamily {
                signature,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn join_family_on_behalf(
        &self,
        member_address: String,
        node_identity_signature: String,
        family_signature: String,
        family_head: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::JoinFamilyOnBehalf {
                member_address,
                node_identity_signature,
                family_signature,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn leave_family(
        &self,
        signature: String,
        family_head: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::LeaveFamily {
                signature,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn leave_family_on_behalf(
        &self,
        member_address: String,
        node_identity_signature: String,
        family_head: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::LeaveFamilyOnBehalf {
                member_address,
                node_identity_signature,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn kick_family_member(
        &self,
        signature: String,
        member: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::KickFamilyMember { signature, member },
            vec![],
        )
        .await
    }

    async fn kick_family_member_on_behalf(
        &self,
        head_address: String,
        signature: String,
        member: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_mixnet_contract(
            fee,
            MixnetExecuteMsg::KickFamilyMemberOnBehalf {
                head_address,
                signature,
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
}

#[async_trait]
impl<C> MixnetSigningClient for NyxdClient<C>
where
    C: SigningCosmWasmClient + Sync + Send + Clone,
{
    async fn execute_mixnet_contract(
        &self,
        fee: Option<Fee>,
        msg: MixnetExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let memo = msg.default_memo();
        self.client
            .execute(
                self.address(),
                self.mixnet_contract_address(),
                &msg,
                fee,
                memo,
                funds,
            )
            .await
    }
}

#[async_trait]
impl<C> MixnetSigningClient for crate::Client<C>
where
    C: SigningCosmWasmClient + Sync + Send + Clone,
{
    async fn execute_mixnet_contract(
        &self,
        fee: Option<Fee>,
        msg: MixnetExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.nyxd.execute_mixnet_contract(fee, msg, funds).await
    }
}
