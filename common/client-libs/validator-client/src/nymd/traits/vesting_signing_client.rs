// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::{Coin, Fee, NymdClient};
use async_trait::async_trait;
use mixnet_contract_common::{Gateway, IdentityKey, IdentityKeyRef, MixNode};
use vesting_contract_common::messages::{ExecuteMsg as VestingExecuteMsg, VestingSpecification};

#[async_trait]
pub trait VestingSigningClient {
    async fn vesting_update_mixnode_config(
        &self,
        profix_margin_percent: u8,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn update_mixnet_address(
        &self,
        address: &str,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_unbond_gateway(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_bond_mixnode(
        &self,
        mix_node: MixNode,
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_unbond_mixnode(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn withdraw_vested_coins(
        &self,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_undelegation(
        &self,
        address: &str,
        mix_identity: IdentityKey,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_delegate_to_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_undelegate_from_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn create_periodic_vesting_account(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send> VestingSigningClient for NymdClient<C> {
    async fn vesting_update_mixnode_config(
        &self,
        profit_margin_percent: u8,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UpdateMixnodeConfig {
            profit_margin_percent,
        };
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
    ) -> Result<ExecuteResult, NymdError> {
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
    ) -> Result<ExecuteResult, NymdError> {
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

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError> {
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
    ) -> Result<ExecuteResult, NymdError> {
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
        owner_signature: &str,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::BondMixnode {
            mix_node,
            owner_signature: owner_signature.to_string(),
            amount: pledge.into(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address(),
                &req,
                fee,
                "VestingContract::BondMixnode",
                vec![],
            )
            .await
    }

    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError> {
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
    ) -> Result<ExecuteResult, NymdError> {
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
    ) -> Result<ExecuteResult, NymdError> {
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
        mix_identity: IdentityKey,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        todo!()
        // let req = VestingExecuteMsg::TrackUndelegation {
        //     owner: address.to_string(),
        //     mix_identity,
        //     amount: amount.into(),
        // };
        // self.client
        //     .execute(
        //         self.address(),
        //         self.vesting_contract_address(),
        //         &req,
        //         fee,
        //         "VestingContract::TrackUndelegation",
        //         vec![],
        //     )
        //     .await
    }
    async fn vesting_delegate_to_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        todo!()
        // let req = VestingExecuteMsg::DelegateToMixnode {
        //     mix_identity: mix_identity.into(),
        //     amount: amount.into(),
        // };
        // self.client
        //     .execute(
        //         self.address(),
        //         self.vesting_contract_address(),
        //         &req,
        //         fee,
        //         "VestingContract::DelegateToMixnode",
        //         vec![],
        //     )
        //     .await
    }

    async fn vesting_undelegate_from_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        todo!()
        // let req = VestingExecuteMsg::UndelegateFromMixnode {
        //     mix_identity: mix_identity.into(),
        // };
        // self.client
        //     .execute(
        //         self.address(),
        //         self.vesting_contract_address(),
        //         &req,
        //         fee,
        //         "VestingContract::UndelegateFromMixnode",
        //         vec![],
        //     )
        //     .await
    }

    async fn create_periodic_vesting_account(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::CreateAccount {
            owner_address: owner_address.to_string(),
            staking_address,
            vesting_spec,
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
