// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::state::ExplorerApiStateContext;
use nym_explorer_api_requests::{
    NymNodeWithDescriptionAndLocation, NymNodeWithDescriptionAndLocationAndDelegations,
    NymVestingAccount, PrettyDetailedGatewayBond,
};
use nym_mixnet_contract_common::{Addr, Coin, NodeId};
use nym_validator_client::nyxd::AccountId;
use okapi::openapi3::OpenApi;
use rocket::response::status::NotFound;
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{Route, State};
use rocket_okapi::settings::OpenApiSettings;
use std::collections::HashMap;
use std::str::FromStr;

pub fn unstable_temp_nymnodes_make_default_routes(
    settings: &OpenApiSettings,
) -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![settings: all_gateways, all_nym_nodes, get_nym_node_by_id, get_account_by_addr]
}

#[openapi(tag = "UNSTABLE")]
#[get("/gateways")]
pub(crate) async fn all_gateways(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<PrettyDetailedGatewayBond>> {
    let mut gateways = state.inner.gateways.get_detailed_gateways().await;
    gateways.append(&mut state.inner.nymnodes.pretty_gateways().await);

    Json(gateways)
}

#[openapi(tag = "UNSTABLE")]
#[get("/nym-nodes")]
pub(crate) async fn all_nym_nodes(
    state: &State<ExplorerApiStateContext>,
) -> Json<Vec<NymNodeWithDescriptionAndLocation>> {
    let nodes = state
        .inner
        .nymnodes
        .get_bonded_nymnodes_with_description_and_location()
        .await;
    Json(nodes.values().cloned().collect())
}

#[openapi(tag = "UNSTABLE")]
#[get("/nym-nodes/<node_id>")]
pub(crate) async fn get_nym_node_by_id(
    node_id: NodeId,
    state: &State<ExplorerApiStateContext>,
) -> Json<Option<NymNodeWithDescriptionAndLocationAndDelegations>> {
    let nodes = state
        .inner
        .nymnodes
        .get_bonded_nymnodes_with_description_and_location()
        .await;
    Json(match nodes.get(&node_id).cloned() {
        None => None,
        Some(node) => {
            let delegations = state.inner.get_delegations_by_node(node_id).await.ok();
            Some(NymNodeWithDescriptionAndLocationAndDelegations {
                node_id: node.node_id,
                contract_node_type: node.contract_node_type,
                description: node.description,
                bond_information: node.bond_information,
                rewarding_details: node.rewarding_details,
                location: node.location,
                delegations,
            })
        }
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NyxAccountDelegationDetails {
    pub node_id: NodeId,
    pub delegated: Coin,
    pub height: u64,
    pub proxy: Option<Addr>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NyxAccountDelegationRewardDetails {
    pub node_id: NodeId,
    pub rewards: Coin,
    pub amount_staked: Coin,
    pub node_still_fully_bonded: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct NyxAccountDetails {
    pub address: String,
    pub balances: Vec<Coin>,
    pub total_value: Coin,
    pub delegations: Vec<NyxAccountDelegationDetails>,
    pub accumulated_rewards: Vec<NyxAccountDelegationRewardDetails>,
    pub total_delegations: Coin,
    pub claimable_rewards: Coin,
    pub vesting_account: Option<NymVestingAccount>,
}

#[openapi(tag = "UNSTABLE")]
#[get("/account/<addr>")]
pub(crate) async fn get_account_by_addr(
    addr: String,
    state: &State<ExplorerApiStateContext>,
) -> Result<Json<NyxAccountDetails>, NotFound<String>> {
    match AccountId::from_str(&addr) {
        Ok(address) => {
            let mut total_value = 0u128;

            // 1. get balances of chain tokens
            let balances: Vec<Coin> = state
                .inner
                .get_balance(&address)
                .await?
                .into_iter()
                .map(|c| {
                    if c.denom == "unym" {
                        total_value += c.amount;
                    }
                    c.into()
                })
                .collect();

            // 2. get list of delegations (history)
            let delegations: Vec<NyxAccountDelegationDetails> = state
                .inner
                .get_delegations(&address)
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
            let mut rewards_map: HashMap<&NodeId, NyxAccountDelegationRewardDetails> =
                HashMap::new();
            for d in &delegations {
                if rewards_map.contains_key(&d.node_id) {
                    continue;
                }

                if let Ok(r) = state
                    .inner
                    .get_delegation_rewards(&address, &d.node_id, &d.proxy)
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
            let vesting_account = state
                .inner
                .get_vesting_balance(&address)
                .await
                .unwrap_or_default();

            if let Some(vesting_account) = vesting_account.clone() {
                total_value += vesting_account.locked.amount.u128();
                total_value += vesting_account.spendable.amount.u128();
            }

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
            }))
        }
        Err(_e) => Err(NotFound("Account not found".to_string())),
    }
}
