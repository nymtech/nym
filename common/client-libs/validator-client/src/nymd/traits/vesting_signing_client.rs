// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::signing_client::SigningCosmWasmClient;
use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::fee::helpers::Operation;
use crate::nymd::{cosmwasm_coin_to_cosmos_coin, NymdClient};
use async_trait::async_trait;
use cosmwasm_std::Coin;
use mixnet_contract_common::{Gateway, IdentityKey, IdentityKeyRef, MixNode};
use vesting_contract::messages::{ExecuteMsg as VestingExecuteMsg, VestingSpecification};

#[async_trait]
pub trait VestingSigningClient {
    async fn update_mixnet_address(&self, address: &str) -> Result<ExecuteResult, NymdError>;

    async fn vesting_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: Coin,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_unbond_gateway(&self) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_unbond_gateway(
        &self,
        owner: &str,
        amount: Coin,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_bond_mixnode(
        &self,
        mix_node: MixNode,
        owner_signature: &str,
        pledge: Coin,
    ) -> Result<ExecuteResult, NymdError>;
    async fn vesting_unbond_mixnode(&self) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_unbond_mixnode(
        &self,
        owner: &str,
        amount: Coin,
    ) -> Result<ExecuteResult, NymdError>;

    async fn withdraw_vested_coins(&self, amount: Coin) -> Result<ExecuteResult, NymdError>;

    async fn vesting_track_undelegation(
        &self,
        address: &str,
        mix_identity: IdentityKey,
        amount: Coin,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_delegate_to_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        amount: &Coin,
    ) -> Result<ExecuteResult, NymdError>;

    async fn vesting_undelegate_from_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn create_periodic_vesting_account(
        &self,
        owner_address: &str,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        amount: Coin,
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C: SigningCosmWasmClient + Sync + Send> VestingSigningClient for NymdClient<C> {
    async fn vesting_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: &str,
        pledge: Coin,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::BondGateway);
        let req = VestingExecuteMsg::BondGateway {
            gateway,
            owner_signature: owner_signature.to_string(),
            amount: pledge,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::BondGateway",
                vec![],
            )
            .await
    }

    async fn vesting_unbond_gateway(&self) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::UnbondGateway);
        let req = VestingExecuteMsg::UnbondGateway {};
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
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
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::TrackUnbondGateway);
        let req = VestingExecuteMsg::TrackUnbondGateway {
            owner: owner.to_string(),
            amount,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
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
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::BondMixnode);
        let req = VestingExecuteMsg::BondMixnode {
            mix_node,
            owner_signature: owner_signature.to_string(),
            amount: pledge,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::BondMixnode",
                vec![],
            )
            .await
    }

    async fn vesting_unbond_mixnode(&self) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::UnbondMixnode);
        let req = VestingExecuteMsg::UnbondMixnode {};
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
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
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::TrackUnbondMixnode);
        let req = VestingExecuteMsg::TrackUnbondMixnode {
            owner: owner.to_string(),
            amount,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::TrackUnbondMixnode",
                vec![],
            )
            .await
    }

    async fn withdraw_vested_coins(&self, amount: Coin) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::WithdrawVestedCoins);
        let req = VestingExecuteMsg::WithdrawVestedCoins { amount };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
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
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::TrackUndelegation);
        let req = VestingExecuteMsg::TrackUndelegation {
            owner: address.to_string(),
            mix_identity,
            amount,
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::TrackUndelegation",
                vec![],
            )
            .await
    }
    async fn vesting_delegate_to_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
        amount: &Coin,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::DelegateToMixnode);
        let req = VestingExecuteMsg::DelegateToMixnode {
            mix_identity: mix_identity.into(),
            amount: amount.clone(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::DeledateToMixnode",
                vec![],
            )
            .await
    }
    async fn vesting_undelegate_from_mixnode<'a>(
        &self,
        mix_identity: IdentityKeyRef<'a>,
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::UndelegateFromMixnode);
        let req = VestingExecuteMsg::UndelegateFromMixnode {
            mix_identity: mix_identity.into(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UndelegateFromMixnode",
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
    ) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::CreatePeriodicVestingAccount);
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
                vec![cosmwasm_coin_to_cosmos_coin(amount)],
            )
            .await
    }

    async fn update_mixnet_address(&self, address: &str) -> Result<ExecuteResult, NymdError> {
        let fee = self.operation_fee(Operation::UpdateMixnetAddress);
        let req = VestingExecuteMsg::UpdateMixnetAddress {
            address: address.to_string(),
        };
        self.client
            .execute(
                self.address(),
                self.vesting_contract_address()?,
                &req,
                fee,
                "VestingContract::UpdateMixnetAddress",
                vec![],
            )
            .await
    }
}
