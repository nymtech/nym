// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::{
    node_status_api::models::{AxumErrorResponse, AxumResult},
    support::http::state::AppState,
    unstable_routes::models::{
        NymVestingAccount, NyxAccountDelegationDetails, NyxAccountDelegationRewardDetails,
        NyxAccountDetails,
    },
};
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use cosmwasm_std::{Coin, Decimal};
use nym_mixnet_contract_common::NodeRewarding;
use nym_topology::NodeId;
use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};
use tracing::{error, instrument, warn};
use utoipa::ToSchema;

pub(crate) fn routes() -> Router<AppState> {
    Router::new().route("/:address", get(address))
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct AddressQueryParam {
    #[serde(default)]
    pub address: String,
}

#[utoipa::path(
    tag = "Unstable",
    get,
    path = "/{address}",
    context_path = "/v1/unstable/account",
    responses(
        (status = 200, body = NyxAccountDetails)
    ),
    params(AddressQueryParam)
)]
#[instrument(level = "info", skip_all, fields(address=address))]
async fn address(
    Path(AddressQueryParam { address }): Path<AddressQueryParam>,
    State(state): State<AppState>,
) -> AxumResult<Json<NyxAccountDetails>> {
    let account_id = AccountId::from_str(&address).map_err(|err| {
        error!("{err}");
        AxumErrorResponse::not_found(&address)
    })?;

    let mut collector = AddressDataCollector::new(state, account_id.clone());

    // ==> get balances of chain tokens <==
    let balance = collector.get_address_balance().await?;

    // ==> get list of delegations (history) <==
    let delegation_data = collector.get_delegations().await?;

    // ==> get the current reward for each active delegation <==
    // calculate rewards from nodes this delegator delegated to
    let accumulated_rewards = collector.calculate_rewards(&delegation_data).await?;

    // ==> get vesting account details (if any) <==
    // (none because everyone has already fully vested)
    let vesting_account: Option<NymVestingAccount> = None;

    // if let Some(vesting_account) = &vesting_account {
    //     total_value += vesting_account.locked.amount.u128();
    //     total_value += vesting_account.spendable.amount.u128();
    // }

    // ==> convert totals <==
    let claimable_rewards = collector.claimable_rewards();
    let total_value = collector.total_value();
    let total_delegations = collector.total_delegations();
    let operator_rewards = collector.operator_rewards();

    Ok(Json(NyxAccountDetails {
        address: account_id.to_string(),
        balance: balance.into(),
        delegations: delegation_data
            .delegations
            .into_iter()
            .map(|d| NyxAccountDelegationDetails {
                delegated: d.amount,
                height: d.height,
                node_id: d.node_id,
                proxy: d.proxy,
            })
            .collect(),
        accumulated_rewards,
        total_delegations,
        claimable_rewards,
        total_value,
        vesting_account,
        operator_rewards,
    }))
}

struct AddressDataCollector {
    app_state: AppState,
    account_id: AccountId,
    total_value: u128,
    operator_rewards: u128,
    claimable_rewards: u128,
    total_delegations: u128,
    base_denom: String,
}

impl AddressDataCollector {
    fn new(app_state: AppState, account_id: AccountId) -> Self {
        let base_denom = app_state
            .network_details()
            .network
            .chain_details
            .mix_denom
            .base
            .to_string();
        Self {
            app_state,
            account_id,
            total_value: 0,
            operator_rewards: 0,
            claimable_rewards: 0,
            total_delegations: 0,
            base_denom,
        }
    }

    async fn get_address_balance(&mut self) -> AxumResult<nym_validator_client::nyxd::Coin> {
        let balance = self
            .app_state
            .nyxd_client
            .get_address_balance(&self.account_id, &self.base_denom)
            .await?
            .unwrap_or_else(|| nym_validator_client::nyxd::Coin::new(0u128, &self.base_denom));
        self.total_value += balance.amount;

        Ok(balance)
    }

    async fn get_delegations(&mut self) -> AxumResult<AddressDelegationInfo> {
        let state = self.app_state.clone();
        let og_delegations = state
            .nyxd_client
            .get_all_delegator_delegations(&self.account_id)
            .await?;

        let delegated_to_nodes = og_delegations
            .iter()
            .map(|d| d.node_id)
            .collect::<HashSet<_>>();

        let nym_nodes = state
            .nym_contract_cache()
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
                if delegated_to_nodes.contains(&node_details.node_id()) {
                    Some((
                        node_details.node_id(),
                        // avoid cloning node data which we don't need
                        (
                            node_details.rewarding_details.clone(),
                            node_details.is_unbonding(),
                        ),
                    ))
                } else {
                    None
                }
            })
            .collect::<HashMap<_, _>>();

        Ok(AddressDelegationInfo {
            delegations: og_delegations,
            delegated_to_nodes: nym_nodes,
        })
    }

    async fn calculate_rewards(
        &mut self,
        delegation_data: &AddressDelegationInfo,
    ) -> AxumResult<Vec<NyxAccountDelegationRewardDetails>> {
        let mut accumulated_rewards = Vec::new();
        for delegation in delegation_data.delegations.iter() {
            let node_id = &delegation.node_id;

            if let Some((rewarding_details, is_unbonding)) =
                delegation_data.delegated_to_nodes.get(node_id)
            {
                match rewarding_details.determine_delegation_reward(delegation) {
                    Ok(delegation_reward) => {
                        let reward = NyxAccountDelegationRewardDetails {
                            node_id: delegation.node_id,
                            rewards: decimal_to_coin(delegation_reward, &self.base_denom),
                            amount_staked: delegation.amount.clone(),
                            node_still_fully_bonded: !is_unbonding,
                        };
                        // 4. sum the rewards and delegations
                        self.total_delegations += delegation.amount.amount.u128();
                        self.total_value += delegation.amount.amount.u128();
                        self.total_value += reward.rewards.amount.u128();
                        self.claimable_rewards += reward.rewards.amount.u128();

                        accumulated_rewards.push(reward);
                    }
                    Err(err) => {
                        warn!(
                            "Couldn't determine delegations for {} on node {}: {}",
                            &self.account_id, node_id, err
                        )
                    }
                }
            }
        }

        Ok(accumulated_rewards)
    }

    fn claimable_rewards(&self) -> Coin {
        Coin::new(self.claimable_rewards, self.base_denom.to_string())
    }

    fn total_value(&self) -> Coin {
        Coin::new(self.total_value, self.base_denom.to_string())
    }

    fn total_delegations(&self) -> Coin {
        Coin::new(self.total_delegations, self.base_denom.to_string())
    }

    fn operator_rewards(&self) -> Option<Coin> {
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

struct AddressDelegationInfo {
    delegations: Vec<nym_mixnet_contract_common::Delegation>,
    delegated_to_nodes: HashMap<NodeId, RewardAndBondInfo>,
}

type RewardAndBondInfo = (NodeRewarding, bool);

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
