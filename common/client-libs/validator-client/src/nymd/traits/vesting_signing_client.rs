// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::{Fee, NymdClient};
use async_trait::async_trait;
use cosmrs::Coin as CosmosCoin;
use cosmwasm_std::Coin as CosmWasmCoin;
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

    async fn vesting_bond_gateway<T: Into<CosmWasmCoin> + Send>(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_unbond_gateway<T: Into<CosmWasmCoin> + Send>(
        &self,
        owner: &str,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_bond_mixnode<T: Into<CosmWasmCoin> + Send>(
        &self,
        mix_node: MixNode,
        owner_signature: &str,
        pledge: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_unbond_mixnode<T: Into<CosmWasmCoin> + Send>(
        &self,
        owner: &str,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn withdraw_vested_coins<T: Into<CosmWasmCoin> + Send>(
        &self,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_undelegation<T: Into<CosmWasmCoin> + Send>(
        &self,
        address: &str,
        mix_identity: IdentityKey,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_delegate_to_mixnode<'a, T: Into<CosmWasmCoin> + Send>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_undelegate_from_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn create_periodic_vesting_account<T: Into<CosmosCoin> + Send>(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: T,
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
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UpdateMixnetConfig",
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
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UpdateMixnetAddress",
            )
            .await
    }

    async fn vesting_bond_gateway<T>(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::BondGateway {
            gateway,
            owner_signature: owner_signature.to_string(),
            amount: pledge.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::BondGateway",
            )
            .await
    }

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UnbondGateway {};
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UnbondGateway",
            )
            .await
    }

    async fn vesting_track_unbond_gateway<T>(
        &self,
        owner: &str,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::TrackUnbondGateway {
            owner: owner.to_string(),
            amount: amount.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::TrackUnbondGateway",
            )
            .await
    }

    async fn vesting_bond_mixnode<T>(
        &self,
        mix_node: MixNode,
        owner_signature: &str,
        pledge: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::BondMixnode {
            mix_node,
            owner_signature: owner_signature.to_string(),
            amount: pledge.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::BondMixnode",
            )
            .await
    }

    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UnbondMixnode {};
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UnbondMixnode",
            )
            .await
    }

    async fn vesting_track_unbond_mixnode<T>(
        &self,
        owner: &str,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::TrackUnbondMixnode {
            owner: owner.to_string(),
            amount: amount.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::TrackUnbondMixnode",
            )
            .await
    }
    async fn withdraw_vested_coins<T>(
        &self,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::WithdrawVestedCoins {
            amount: amount.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::WithdrawVested",
            )
            .await
    }
    async fn vesting_track_undelegation<T>(
        &self,
        address: &str,
        mix_identity: IdentityKey,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::TrackUndelegation {
            owner: address.to_string(),
            mix_identity,
            amount: amount.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::TrackUndelegation",
            )
            .await
    }
    async fn vesting_delegate_to_mixnode<'a, T>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmWasmCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::DelegateToMixnode {
            mix_identity: mix_identity.into(),
            amount: amount.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::DelegateToMixnode",
            )
            .await
    }

    async fn vesting_undelegate_from_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::UndelegateFromMixnode {
            mix_identity: mix_identity.into(),
        };
        self.client
            .fundless_execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UndelegateFromMixnode",
            )
            .await
    }

    async fn create_periodic_vesting_account<T>(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: T,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>
    where
        T: Into<CosmosCoin> + Send,
    {
        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier)));
        let req = VestingExecuteMsg::CreateAccount {
            owner_address: owner_address.to_string(),
            staking_address,
            vesting_spec,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::CreatePeriodicVestingAccount",
                vec![amount.into()],
            )
            .await
    }
}
