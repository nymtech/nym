// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
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
use nym_topology::NodeId;
use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
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

    let node_cache = state.nym_contract_cache();
    let nym_nodes = node_cache
        .nym_nodes()
        .await
        .into_iter()
        .map(|node| (node.node_id(), node))
        .collect::<HashMap<_, _>>();

    let state_client = state.nyxd_client;
    let mut total_value = 0u128;

    // 1. get balances of chain tokens
    let balances = state_client
        .get_all_balances(&account_id)
        .await?
        .into_iter()
        .map(|c| {
            if c.denom == "unym" {
                total_value += c.amount;
            }
            c.into()
        })
        .collect::<Vec<Coin>>();

    // 2. get list of delegations (history)
    let og_delegations = state_client
        .get_all_delegator_delegations(&account_id)
        .await?;
    let delegations: Vec<NyxAccountDelegationDetails> = og_delegations
        .iter()
        .map(|d| NyxAccountDelegationDetails {
            delegated: d.amount.clone(),
            height: d.height,
            node_id: d.node_id,
            proxy: d.proxy.clone(),
        })
        .collect();

    // 3. get the current reward for each active delegation
    // calculate rewards from nodes this delegator delegated to
    let mut rewards_map: HashMap<&NodeId, NyxAccountDelegationRewardDetails> = HashMap::new();
    for delegation in og_delegations.iter() {
        let node_id = &delegation.node_id;

        if let Some(nym_node_details) = nym_nodes.get(node_id) {
            match nym_node_details
                .rewarding_details
                .determine_delegation_reward(delegation)
            {
                Ok(delegation_reward) => {
                    rewards_map.insert(
                        node_id,
                        NyxAccountDelegationRewardDetails {
                            node_id: delegation.node_id,
                            rewards: decimal_to_coin(delegation_reward, "unym"),
                            amount_staked: nym_node_details.original_pledge().clone(),
                            node_still_fully_bonded: !nym_node_details.is_unbonding(),
                        },
                    );
                }
                Err(err) => {
                    warn!(
                        "Couldn't determine delegations for {} on node {}: {}",
                        &account_id, node_id, err
                    )
                }
            }
        }
    }

    // 4. make the map of rewards into a vec and sum the rewards and delegations
    let accumulated_rewards: Vec<NyxAccountDelegationRewardDetails> =
        rewards_map.values().cloned().collect();

    let mut claimable_rewards = 0u128;
    let mut total_delegations = 0u128;
    for r in &accumulated_rewards {
        claimable_rewards += r.rewards.amount.u128();
        total_delegations += r.amount_staked.amount.u128();
        total_value += r.rewards.amount.u128();
        total_value += r.amount_staked.amount.u128();
    }

    // 5. get vesting account details (if any): none because everyone has already fully vested
    let vesting_account: Option<NymVestingAccount> = None;

    if let Some(vesting_account) = vesting_account.clone() {
        total_value += vesting_account.locked.amount.u128();
        total_value += vesting_account.spendable.amount.u128();
    }

    // 6. get operator rewards: is this an operator?
    let mut operator_rewards = 0;

    for (_, node_details) in nym_nodes.iter() {
        if address == node_details.bond_information.owner.as_str() {
            let pending_operator_reward = node_details.pending_operator_reward().amount.u128();

            // add operator rewards
            operator_rewards += pending_operator_reward;

            // add to totals
            total_value += pending_operator_reward;
        }
    }

    // 7. convert totals
    let claimable_rewards = Coin::new(claimable_rewards, "unym");
    let total_delegations = Coin::new(total_delegations, "unym");
    let total_value = Coin::new(total_value, "unym");
    let operator_rewards = if operator_rewards > 0 {
        Some(Coin::new(operator_rewards, "unym"))
    } else {
        None
    };

    Ok(Json(NyxAccountDetails {
        address: account_id.to_string(),
        balances,
        delegations,
        accumulated_rewards,
        claimable_rewards,
        total_delegations,
        total_value,
        vesting_account,
        operator_rewards,
    }))
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
