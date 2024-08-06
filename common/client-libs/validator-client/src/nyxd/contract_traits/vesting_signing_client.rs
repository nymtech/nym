// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::signing::MessageSignature;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::gateway::GatewayConfigUpdate;
use nym_mixnet_contract_common::mixnode::{MixNodeConfigUpdate, MixNodeCostParams};
use nym_mixnet_contract_common::{Gateway, MixId, MixNode};
use nym_vesting_contract_common::messages::ExecuteMsg as VestingExecuteMsg;
use nym_vesting_contract_common::{PledgeCap, VestingSpecification};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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
        let req = VestingExecuteMsg::UpdateMixnodeConfig { new_config };
        self.execute_vesting_contract(fee, req, vec![]).await
    }

    async fn vesting_update_gateway_config(
        &self,
        new_config: GatewayConfigUpdate,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::UpdateGatewayConfig { new_config },
            vec![],
        )
        .await
    }

    async fn update_mixnet_address(
        &self,
        address: &str,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::UpdateMixnetAddress {
            address: address.to_string(),
        };
        self.execute_vesting_contract(fee, req, vec![]).await
    }

    async fn vesting_track_decrease_pledge(
        &self,
        owner: String,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::TrackDecreasePledge {
                owner,
                amount: amount.into(),
            },
            Vec::new(),
        )
        .await
    }

    async fn vesting_bond_gateway(
        &self,
        gateway: Gateway,
        owner_signature: MessageSignature,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::BondGateway {
            gateway,
            owner_signature,
            amount: pledge.into(),
        };
        self.execute_vesting_contract(fee, req, vec![]).await
    }

    async fn vesting_unbond_gateway(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::UnbondGateway {};
        self.execute_vesting_contract(fee, req, vec![]).await
    }
    async fn vesting_track_unbond_gateway(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::TrackUnbondGateway {
            owner: owner.to_string(),
            amount: amount.into(),
        };
        self.execute_vesting_contract(fee, req, vec![]).await
    }

    async fn vesting_bond_mixnode(
        &self,
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: MessageSignature,
        pledge: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::BondMixnode {
                mix_node,
                cost_params,
                owner_signature,
                amount: pledge.into(),
            },
            vec![],
        )
        .await
    }

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

    async fn vesting_decrease_pledge(
        &self,
        decrease_by: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::DecreasePledge {
                amount: decrease_by.into(),
            },
            vec![],
        )
        .await
    }

    async fn vesting_unbond_mixnode(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::UnbondMixnode {};
        self.execute_vesting_contract(fee, req, vec![]).await
    }

    async fn vesting_track_unbond_mixnode(
        &self,
        owner: &str,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::TrackUnbondMixnode {
            owner: owner.to_string(),
            amount: amount.into(),
        };
        self.execute_vesting_contract(fee, req, vec![]).await
    }

    async fn withdraw_vested_coins(
        &self,
        amount: Coin,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = VestingExecuteMsg::WithdrawVestedCoins {
            amount: amount.into(),
        };
        self.execute_vesting_contract(fee, req, vec![]).await
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
        let req = VestingExecuteMsg::CreateAccount {
            owner_address: owner_address.to_string(),
            staking_address,
            vesting_spec,
            cap,
        };
        self.execute_vesting_contract(fee, req, vec![amount]).await
    }

    async fn vesting_track_reward(
        &self,
        amount: Coin,
        address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::TrackReward {
                amount: amount.into(),
                address,
            },
            Vec::new(),
        )
        .await
    }

    async fn vesting_withdraw_operator_reward(
        &self,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(fee, VestingExecuteMsg::ClaimOperatorReward {}, Vec::new())
            .await
    }

    async fn vesting_withdraw_delegator_reward(
        &self,
        mix_id: MixId,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::ClaimDelegatorReward { mix_id },
            Vec::new(),
        )
        .await
    }

    async fn vesting_transfer_ownership(
        &self,
        to_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::TransferOwnership { to_address },
            Vec::new(),
        )
        .await
    }

    async fn update_staking_address(
        &self,
        to_address: Option<String>,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::UpdateStakingAddress { to_address },
            Vec::new(),
        )
        .await
    }

    async fn update_locked_pledge_cap(
        &self,
        address: AccountId,
        cap: PledgeCap,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::UpdateLockedPledgeCap {
                address: address.to_string(),
                cap,
            },
            Vec::new(),
        )
        .await
    }

    async fn vesting_create_family(
        &self,
        label: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(fee, VestingExecuteMsg::CreateFamily { label }, vec![])
            .await
    }

    async fn vesting_join_family(
        &self,
        join_permit: MessageSignature,
        family_head: FamilyHead,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(
            fee,
            VestingExecuteMsg::JoinFamily {
                join_permit,
                family_head,
            },
            vec![],
        )
        .await
    }

    async fn vesting_leave_family(
        &self,
        family_head: FamilyHead,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(fee, VestingExecuteMsg::LeaveFamily { family_head }, vec![])
            .await
    }

    async fn vesting_kick_family_member(
        &self,
        member: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.execute_vesting_contract(fee, VestingExecuteMsg::KickFamilyMember { member }, vec![])
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> VestingSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_vesting_contract(
        &self,
        fee: Option<Fee>,
        msg: VestingExecuteMsg,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let vesting_contract_address = &self
            .vesting_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("vesting contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let memo = msg.name().to_string();

        let signer_address = &self.signer_addresses()?[0];
        self.execute(
            signer_address,
            vesting_contract_address,
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
    use nym_vesting_contract_common::ExecuteMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: VestingSigningClient + Send + Sync>(
        client: C,
        msg: VestingExecuteMsg,
    ) {
        match msg {
            VestingExecuteMsg::CreateFamily { label } => {
                client.vesting_create_family(label, None).ignore()
            }
            VestingExecuteMsg::JoinFamily {
                join_permit,
                family_head,
            } => client
                .vesting_join_family(join_permit, family_head, None)
                .ignore(),
            VestingExecuteMsg::LeaveFamily { family_head } => {
                client.vesting_leave_family(family_head, None).ignore()
            }
            VestingExecuteMsg::KickFamilyMember { member } => {
                client.vesting_kick_family_member(member, None).ignore()
            }
            VestingExecuteMsg::TrackReward { amount, address } => client
                .vesting_track_reward(amount.into(), address, None)
                .ignore(),
            VestingExecuteMsg::ClaimOperatorReward {} => {
                client.vesting_withdraw_operator_reward(None).ignore()
            }
            VestingExecuteMsg::ClaimDelegatorReward { mix_id } => client
                .vesting_withdraw_delegator_reward(mix_id, None)
                .ignore(),
            VestingExecuteMsg::UpdateMixnodeCostParams { new_costs } => client
                .vesting_update_mixnode_cost_params(new_costs, None)
                .ignore(),
            VestingExecuteMsg::UpdateMixnodeConfig { new_config } => client
                .vesting_update_mixnode_config(new_config, None)
                .ignore(),
            VestingExecuteMsg::UpdateMixnetAddress { address } => {
                client.update_mixnet_address(&address, None).ignore()
            }
            VestingExecuteMsg::DelegateToMixnode {
                mix_id,
                amount,
                on_behalf_of,
            } => client
                .vesting_delegate_to_mixnode(mix_id, amount.into(), on_behalf_of, None)
                .ignore(),
            VestingExecuteMsg::UndelegateFromMixnode {
                mix_id,
                on_behalf_of,
            } => client
                .vesting_undelegate_from_mixnode(mix_id, on_behalf_of, None)
                .ignore(),
            VestingExecuteMsg::CreateAccount {
                owner_address,
                staking_address,
                vesting_spec,
                cap,
            } => client
                .create_periodic_vesting_account(
                    &owner_address,
                    staking_address,
                    vesting_spec,
                    mock_coin(),
                    cap,
                    None,
                )
                .ignore(),
            VestingExecuteMsg::WithdrawVestedCoins { amount } => {
                client.withdraw_vested_coins(amount.into(), None).ignore()
            }
            VestingExecuteMsg::TrackUndelegation {
                owner,
                mix_id,
                amount,
            } => client
                .vesting_track_undelegation(&owner, mix_id, amount.into(), None)
                .ignore(),
            VestingExecuteMsg::BondMixnode {
                mix_node,
                cost_params,
                owner_signature,
                amount,
            } => client
                .vesting_bond_mixnode(mix_node, cost_params, owner_signature, amount.into(), None)
                .ignore(),
            VestingExecuteMsg::PledgeMore { amount } => {
                client.vesting_pledge_more(amount.into(), None).ignore()
            }
            VestingExecuteMsg::DecreasePledge { amount } => {
                client.vesting_decrease_pledge(amount.into(), None).ignore()
            }
            VestingExecuteMsg::UnbondMixnode {} => client.vesting_unbond_mixnode(None).ignore(),
            VestingExecuteMsg::TrackUnbondMixnode { owner, amount } => client
                .vesting_track_unbond_mixnode(&owner, amount.into(), None)
                .ignore(),
            VestingExecuteMsg::TrackDecreasePledge { owner, amount } => client
                .vesting_track_decrease_pledge(owner, amount.into(), None)
                .ignore(),
            VestingExecuteMsg::BondGateway {
                gateway,
                owner_signature,
                amount,
            } => client
                .vesting_bond_gateway(gateway, owner_signature, amount.into(), None)
                .ignore(),
            VestingExecuteMsg::UnbondGateway {} => client.vesting_unbond_gateway(None).ignore(),
            VestingExecuteMsg::TrackUnbondGateway { owner, amount } => client
                .vesting_track_unbond_gateway(&owner, amount.into(), None)
                .ignore(),
            VestingExecuteMsg::UpdateGatewayConfig { new_config } => client
                .vesting_update_gateway_config(new_config, None)
                .ignore(),
            VestingExecuteMsg::TransferOwnership { to_address } => {
                client.vesting_transfer_ownership(to_address, None).ignore()
            }
            VestingExecuteMsg::UpdateStakingAddress { to_address } => {
                client.update_staking_address(to_address, None).ignore()
            }
            VestingExecuteMsg::UpdateLockedPledgeCap { address, cap } => client
                .update_locked_pledge_cap(address.parse().unwrap(), cap, None)
                .ignore(),
            // those will never be manually called by clients
            ExecuteMsg::TrackMigratedMixnode { .. } => "explicitly_ignored".ignore(),
            ExecuteMsg::TrackMigratedDelegation { .. } => "explicitly_ignored".ignore(),
        };
    }
}
