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
use cosmwasm_std::Coin;
use nym_topology::NodeId;
use nym_validator_client::nyxd::AccountId;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, str::FromStr};
use tracing::{error, instrument};
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
    let address = AccountId::from_str(&address).map_err(|err| {
        error!("{err}");
        AxumErrorResponse::not_found(address)
    })?;

    let state_client = state.nyxd_client;
    let mut total_value = 0u128;

    // 1. get balances of chain tokens
    let balances = state_client
        .get_all_balances(&address)
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
    let delegations: Vec<NyxAccountDelegationDetails> = state_client
        .get_all_delegator_delegations(&address)
        .await?
        .into_iter()
        .map(|d| NyxAccountDelegationDetails {
            delegated: d.amount,
            height: d.height,
            node_id: d.node_id,
            proxy: d.proxy,
        })
        .collect();

    // 3. get the current reward for each active delegation
    let mut rewards_map: HashMap<&NodeId, NyxAccountDelegationRewardDetails> = HashMap::new();
    for d in &delegations {
        if rewards_map.contains_key(&d.node_id) {
            continue;
        }

        if let Ok(r) = state_client
            .get_pending_delegator_reward(
                &address,
                d.node_id,
                d.proxy.clone().map(|d| d.to_string()),
            )
            .await
        {
            if let Some(rewards) = r.amount_earned {
                rewards_map.insert(
                    &d.node_id,
                    NyxAccountDelegationRewardDetails {
                        node_id: d.node_id,
                        rewards,
                        amount_staked: r.amount_staked.unwrap_or_default(),
                        node_still_fully_bonded: r.node_still_fully_bonded,
                    },
                );
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

    // 5. get vesting account details (if present)
    // everyone has already fully vested
    let vesting_account: Option<NymVestingAccount> = None;

    if let Some(vesting_account) = vesting_account.clone() {
        total_value += vesting_account.locked.amount.u128();
        total_value += vesting_account.spendable.amount.u128();
    }

    // 6. get operator rewards
    let operator_rewards: Option<Coin> = if let Ok(operator_rewards_res) =
        state_client.get_pending_operator_reward(&address).await
    {
        if let Some(operator_reward_amount) = &operator_rewards_res.amount_earned {
            total_value += operator_reward_amount.amount.u128();
        }

        operator_rewards_res.amount_earned
    } else {
        None
    };

    // 7. convert totals
    let claimable_rewards = Coin::new(claimable_rewards, "unym");
    let total_delegations = Coin::new(total_delegations, "unym");
    let total_value = Coin::new(total_value, "unym");

    Ok(Json(NyxAccountDetails {
        address: address.to_string(),
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
