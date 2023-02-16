// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nyxd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, NyxdClient};
use async_trait::async_trait;
use nym_mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use nym_mixnet_contract_common::{Gateway, MixId, MixNode};
use nym_vesting_contract_common::messages::{
    ExecuteMsg as VestingExecuteMsg, VestingSpecification,
};
use nym_vesting_contract_common::PledgeCap;

#[async_trait]
pub trait VestingSigningClient {
    async fn execute_vesting_contract(
        &self,
        fee: Option<Fee>,
        msg: VestingExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_update_mixnode_cost_params(
        &self,
        new_costs: MixNodeCostParams,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_update_mixnode_config(
        &self,
        new_config: MixNodeConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn update_mixnet_address(
        &self,
        address: &str,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_track_unbond_gateway(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_bond_mixnode(
        &self,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_pledge_more(
        &self,
        additional_pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::PledgeMore {
                amount: additional_pledge.into(),
            },
            vec![],
        )
        .await
    }

    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_track_unbond_mixnode(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn withdraw_vested_coins(
        &self,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_track_undelegation(
        &self,
        address: &str,
        mix_id: MixId,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_delegate_to_mixnode(
        &self,
        mix_id: MixId,
        amount: Coin,
        on_behalf_of: Option<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn vesting_undelegate_from_mixnode(
        &self,
        mix_id: MixId,
        on_behalf_of: Option<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn create_periodic_vesting_account(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: Coin,
        cap: Option<PledgeCap>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send + Clone> VestingSigningClient for NyxdClient<C> {
    async fn execute_vesting_contract(
        &self,
        fee: Option<Fee>,
        msg: VestingExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let memo = msg.name().to_string();
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &msg,
                fee,
                memo,
                funds,
            )
            .await
    }

    async fn vesting_update_mixnode_cost_params(
        &self,
        new_costs: MixNodeCostParams,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::UpdateMixnodeCostParams { new_costs },
            vec![],
        )
        .await
    }

    async fn vesting_update_mixnode_config(
        &self,
        new_config: MixNodeConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UpdateMixnodeConfig { new_config };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::UpdateMixnetConfig",
                vec![],
            )
            .await
    }

    async fn update_mixnet_address(
        &self,
        address: &str,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UpdateMixnetAddress {
            address: address.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::UpdateMixnetAddress",
                vec![],
            )
            .await
    }

    async fn vesting_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::BondGateway {
            gateway,
            owner_signature: owner_signature.to_string(),
            amount: pledge.into(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::BondGateway",
                vec![],
            )
            .await
    }

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UnbondGateway {};
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::UnbondGateway",
                vec![],
            )
            .await
    }

    async fn vesting_track_unbond_gateway(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::TrackUnbondGateway {
            owner: owner.to_string(),
            amount: amount.into(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::TrackUnbondGateway",
                vec![],
            )
            .await
    }

    async fn vesting_bond_mixnode(
        &self,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::BondMixnode {
                mix_node,
                cost_params,
                owner_signature: owner_signature.to_string(),
                amount: pledge.into(),
            },
            vec![],
        )
        .await
    }

    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UnbondMixnode {};
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::UnbondMixnode",
                vec![],
            )
            .await
    }

    async fn vesting_track_unbond_mixnode(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::TrackUnbondMixnode {
            owner: owner.to_string(),
            amount: amount.into(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::TrackUnbondMixnode",
                vec![],
            )
            .await
    }

    async fn withdraw_vested_coins(
        &self,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::WithdrawVestedCoins {
            amount: amount.into(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::WithdrawVested",
                vec![],
            )
            .await
    }

    async fn vesting_track_undelegation(
        &self,
        address: &str,
        mix_id: MixId,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::TrackUndelegation {
                owner: address.to_string(),
                mix_id,
                amount: amount.into(),
            },
            vec![],
        )
        .await
    }

    async fn vesting_delegate_to_mixnode(
        &self,
        mix_id: MixId,
        amount: Coin,
        on_behalf_of: Option<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::DelegateToMixnode {
                mix_id,
                amount: amount.into(),
                on_behalf_of,
            },
            vec![],
        )
        .await
    }

    async fn vesting_undelegate_from_mixnode(
        &self,
        mix_id: MixId,
        on_behalf_of: Option<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::UndelegateFromMixnode {
                mix_id,
                on_behalf_of,
            },
            vec![],
        )
        .await
    }

    async fn create_periodic_vesting_account(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: Coin,
        cap: Option<PledgeCap>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::CreateAccount {
            owner_address: owner_address.to_string(),
            staking_address,
            vesting_spec,
            cap,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::CreatePeriodicVestingAccount",
                vec![amount],
            )
            .await
    }
}
