// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::unstable_routes::v1::account::models::NyxAccountDelegationRewardDetails;
use crate::{
    node_status_api::models::{AxumErrorResponse, AxumResult},
    nym_contract_cache::cache::NymContractCache,
};
use cosmwasm_std::{Coin, Decimal};
use nym_mixnet_contract_common::NodeRewarding;
use nym_validator_client::nyxd::AccountId;
use std::collections::{HashMap, HashSet};
use tracing::error;

pub(crate) struct AddressDataCollector {
    nyxd_client: crate::nyxd::Client,
    nym_contract_cache: NymContractCache,
    account_id: AccountId,
    total_value: u128,
    operator_rewards: u128,
    claimable_rewards: u128,
    total_delegations: u128,
    base_denom: String,
}

impl AddressDataCollector {
    pub(crate) fn new(
        nyxd_client: crate::nyxd::Client,
        nym_contract_cache: NymContractCache,
        base_denom: String,
        account_id: AccountId,
    ) -> Self {
        Self {
            nyxd_client,
            nym_contract_cache,
            base_denom,
            account_id,
            total_value: 0,
            operator_rewards: 0,
            claimable_rewards: 0,
            total_delegations: 0,
        }
    }

    pub(crate) async fn get_address_balance(
        &mut self,
    ) -> AxumResult<nym_validator_client::nyxd::Coin> {
        let balance = self
            .nyxd_client
            .get_address_balance(&self.account_id, &self.base_denom)
            .await?
            .unwrap_or_else(|| nym_validator_client::nyxd::Coin::new(0u128, &self.base_denom));
        self.total_value += balance.amount;

        Ok(balance)
    }

    pub(crate) async fn get_delegations(&mut self) -> AxumResult<Vec<AddressDelegationInfo>> {
        let og_delegations = self
            .nyxd_client
            .get_all_delegator_delegations(&self.account_id)
            .await?
            .into_iter()
            .map(|delegation| (delegation.node_id, delegation))
            .collect::<HashMap<_, _>>();

        let mut node_delegation_info = Vec::new();

        let delegated_to_nodes_bonded = self
            .nym_contract_cache
            .all_cached_nym_nodes()
            .await
            .ok_or_else(AxumErrorResponse::service_unavailable)?
            .iter()
            .filter_map(|node_details| {
                // is this an operator of this node?
                if self.account_id.to_string() == node_details.bond_information.owner.as_str() {
                    let pending_operator_reward =
                        node_details.pending_operator_reward().amount.u128();

                    // add operator rewards
                    self.operator_rewards += pending_operator_reward;

                    // add to totals
                    self.total_value += pending_operator_reward;
                }
                if let Some(delegation) = og_delegations.get(&node_details.node_id()) {
                    node_delegation_info.push(AddressDelegationInfo {
                        details: delegation.clone(),
                        node_reward_info: NodeBondStatus::Bonded {
                            rewarding_info: node_details.rewarding_details.to_owned(),
                            unbonding: node_details.is_unbonding(),
                        },
                    });

                    Some(node_details.node_id())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        for (node_id, delegation) in og_delegations {
            if !delegated_to_nodes_bonded.contains(&node_id) {
                node_delegation_info.push(AddressDelegationInfo {
                    details: delegation.clone(),
                    node_reward_info: NodeBondStatus::UnBonded,
                });
            }
        }

        Ok(node_delegation_info)
    }

    pub(crate) async fn calculate_rewards(
        &mut self,
        delegation_data: &Vec<AddressDelegationInfo>,
    ) -> AxumResult<Vec<NyxAccountDelegationRewardDetails>> {
        let mut accumulated_rewards = Vec::new();
        for delegation in delegation_data {
            let node_id = delegation.details.node_id;

            match &delegation.node_reward_info {
                NodeBondStatus::Bonded {
                    rewarding_info,
                    unbonding,
                } => {
                    match rewarding_info.determine_delegation_reward(&delegation.details) {
                        Ok(delegation_reward) => {
                            let reward = NyxAccountDelegationRewardDetails {
                                node_id,
                                rewards: decimal_to_coin(delegation_reward, &self.base_denom),
                                amount_staked: delegation.details.amount.clone(),
                                node_still_fully_bonded: !unbonding,
                            };
                            // 4. sum the rewards and delegations
                            self.total_delegations += delegation.details.amount.amount.u128();
                            self.total_value += delegation.details.amount.amount.u128();
                            self.total_value += reward.rewards.amount.u128();
                            self.claimable_rewards += reward.rewards.amount.u128();

                            accumulated_rewards.push(reward);
                        }
                        Err(err) => {
                            error!(
                                "Couldn't determine delegations for {} on node {}: {}",
                                &self.account_id, node_id, err
                            )
                        }
                    }
                }
                NodeBondStatus::UnBonded => {
                    // directory cache doesn't store node details required to
                    // calculate rewarding for unbonded nodes
                }
            }
        }

        Ok(accumulated_rewards)
    }

    pub(crate) fn claimable_rewards(&self) -> Coin {
        Coin::new(self.claimable_rewards, self.base_denom.to_string())
    }

    pub(crate) fn total_value(&self) -> Coin {
        Coin::new(self.total_value, self.base_denom.to_string())
    }

    pub(crate) fn total_delegations(&self) -> Coin {
        Coin::new(self.total_delegations, self.base_denom.to_string())
    }

    pub(crate) fn operator_rewards(&self) -> Option<Coin> {
        if self.operator_rewards > 0 {
            Some(Coin::new(
                self.operator_rewards,
                self.base_denom.to_string(),
            ))
        } else {
            None
        }
    }
}

pub(crate) struct AddressDelegationInfo {
    details: nym_mixnet_contract_common::Delegation,
    node_reward_info: NodeBondStatus,
}

impl AddressDelegationInfo {
    pub(crate) fn details(&self) -> &nym_mixnet_contract_common::Delegation {
        &self.details
    }

    pub(crate) fn is_node_bonded(&self) -> bool {
        matches!(self.node_reward_info, NodeBondStatus::Bonded { .. })
    }
}

pub(crate) enum NodeBondStatus {
    Bonded {
        rewarding_info: NodeRewarding,
        unbonding: bool,
    },
    UnBonded,
}

fn decimal_to_coin(decimal: Decimal, denom: impl Into<String>) -> Coin {
    Coin::new(decimal.to_uint_floor(), denom)
}

#[cfg(test)]
mod test {

    use super::*;

    #[tokio::test]
    async fn decimal_to_coin_test() {
        let test_values = [
            (1234, 0, 1234),
            (1234, 2, 12),
            (1_234_000_000_000_000u128, 6, 1_234_000_000u128),
        ];

        for (amount, decimal_places, coin_amount) in test_values {
            let decimal =
                Decimal::from_atomics(cosmwasm_std::Uint128::new(amount), decimal_places).unwrap();
            let coin_from_decimal = decimal_to_coin(decimal, "unym");
            assert_eq!(coin_from_decimal, Coin::new(coin_amount, "unym"));
        }
    }
}
