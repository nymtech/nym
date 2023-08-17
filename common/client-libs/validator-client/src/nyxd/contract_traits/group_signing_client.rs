// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cw4::Member;
use nym_group_contract_common::msg::ExecuteMsg as GroupExecuteMsg;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GroupSigningClient {
    async fn execute_group_contract(
        &self,
        fee: Option<Fee>,
        msg: GroupExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn update_admin(
        &self,
        admin: Option<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_group_contract(
            fee,
            GroupExecuteMsg::UpdateAdmin { admin },
            "GroupExecuteMsg::UpdateAdmin".to_string(),
            vec![],
        )
        .await
    }

    async fn update_members(
        &self,
        add: Vec<Member>,
        remove: Vec<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_group_contract(
            fee,
            GroupExecuteMsg::UpdateMembers { add, remove },
            "GroupExecuteMsg::UpdateMembers".to_string(),
            vec![],
        )
        .await
    }

    async fn add_hook(&self, addr: String, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        self.execute_group_contract(
            fee,
            GroupExecuteMsg::AddHook { addr },
            "GroupExecuteMsg::AddHook".to_string(),
            vec![],
        )
        .await
    }

    async fn remove_hook(
        &self,
        addr: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_group_contract(
            fee,
            GroupExecuteMsg::RemoveHook { addr },
            "GroupExecuteMsg::RemoveHook".to_string(),
            vec![],
        )
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> GroupSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_group_contract(
        &self,
        fee: Option<Fee>,
        msg: GroupExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let group_contract_address = self
            .group_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("group contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            group_contract_address,
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
    fn all_execute_variants_are_covered<C: GroupSigningClient + Send + Sync>(
        client: C,
        msg: GroupExecuteMsg,
    ) {
        match msg {
            GroupExecuteMsg::UpdateAdmin { admin } => client.update_admin(admin, None).ignore(),
            GroupExecuteMsg::UpdateMembers { remove, add } => {
                client.update_members(add, remove, None).ignore()
            }
            GroupExecuteMsg::AddHook { addr } => client.add_hook(addr, None).ignore(),
            GroupExecuteMsg::RemoveHook { addr } => client.remove_hook(addr, None).ignore(),
        };
    }
}
