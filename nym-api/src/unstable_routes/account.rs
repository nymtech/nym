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
    let base_denom = &state.network_details().network.chain_details.mix_denom.base;

    let state_client = &state.nyxd_client;
    let mut total_value = 0u128;

    // 1. get balances of chain tokens
    let balance = state_client
        .get_address_balance(&account_id, base_denom)
        .await?
        .unwrap_or_else(|| nym_validator_client::nyxd::Coin::new(0u128, base_denom));
    total_value += balance.amount;

    // 2. get list of delegations (history)
    let og_delegations = state_client
        .get_all_delegator_delegations(&account_id)
        .await?;

    let delegated_to_nodes = og_delegations
        .iter()
        .map(|d| d.node_id)
        .collect::<HashSet<_>>();

    let mut operator_rewards = 0;
    let nym_nodes = state
        .nym_contract_cache()
        .all_cached_nym_nodes()
        .await
        .ok_or_else(AxumErrorResponse::service_unavailable)?
        .iter()
        .filter_map(|node_details| {
            // is this an operator of this node?
            if account_id.to_string() == node_details.bond_information.owner.as_str() {
                let pending_operator_reward = node_details.pending_operator_reward().amount.u128();

                // add operator rewards
                operator_rewards += pending_operator_reward;

                // add to totals
                total_value += pending_operator_reward;
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

    // 3. get the current reward for each active delegation
    // calculate rewards from nodes this delegator delegated to
    let mut claimable_rewards = 0u128;
    let mut total_delegations = 0u128;

    let mut accumulated_rewards = Vec::new();
    for delegation in og_delegations.iter() {
        let node_id = &delegation.node_id;

        if let Some((rewarding_details, is_unbonding)) = nym_nodes.get(node_id) {
            match rewarding_details.determine_delegation_reward(delegation) {
                Ok(delegation_reward) => {
                    let reward = NyxAccountDelegationRewardDetails {
                        node_id: delegation.node_id,
                        rewards: decimal_to_coin(delegation_reward, base_denom),
                        amount_staked: delegation.amount.clone(),
                        node_still_fully_bonded: !is_unbonding,
                    };
                    // 4. sum the rewards and delegations
                    total_delegations += delegation.amount.amount.u128();
                    total_value += delegation.amount.amount.u128();
                    total_value += reward.rewards.amount.u128();
                    claimable_rewards += reward.rewards.amount.u128();

                    accumulated_rewards.push(reward);
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
    drop(nym_nodes);

    // 5. get vesting account details (if any): none because everyone has already fully vested
    let vesting_account: Option<NymVestingAccount> = None;

    if let Some(vesting_account) = &vesting_account {
        total_value += vesting_account.locked.amount.u128();
        total_value += vesting_account.spendable.amount.u128();
    }

    // 6. convert totals
    let claimable_rewards = Coin::new(claimable_rewards, base_denom);
    let total_value = Coin::new(total_value, base_denom);
    let total_delegations = Coin::new(total_delegations, base_denom);
    let operator_rewards = if operator_rewards > 0 {
        Some(Coin::new(operator_rewards, base_denom))
    } else {
        None
    };

    Ok(Json(NyxAccountDetails {
        address: account_id.to_string(),
        balance: balance.into(),
        delegations: og_delegations
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
