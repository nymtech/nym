// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::msg::MigrateMsg;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    Addr, Deps, DepsMut, Env, MessageInfo, QueryResponse, Response, StdError, StdResult, Storage,
};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex};
use nym_coconut_dkg_common::dealer::DealerRegistrationDetails;
use nym_coconut_dkg_common::types::{Epoch, EpochId, EpochState, NodeIndex};
use nym_coconut_dkg_common::verification_key::ContractVKShare;

pub(crate) type Dealer<'a> = &'a Addr;

pub(crate) const CURRENT_EPOCH: Item<'_, Epoch> = Item::new("current_epoch");

pub const THRESHOLD: Item<u64> = Item::new("threshold");

pub const EPOCH_THRESHOLDS: Map<EpochId, u64> = Map::new("epoch_thresholds");

pub(crate) const NODE_INDEX_COUNTER: Item<NodeIndex> = Item::new("node_index_counter");

// use the same storage types as the actual DKG contract
pub(crate) const DEALERS_INDICES: Map<Dealer, NodeIndex> = Map::new("dealer_index");

pub(crate) const EPOCH_DEALERS_MAP: Map<(EpochId, Dealer), DealerRegistrationDetails> =
    Map::new("epoch_dealers");

type VKShareKey<'a> = (&'a Addr, EpochId);

pub(crate) struct VkShareIndex<'a> {
    pub(crate) epoch_id: MultiIndex<'a, EpochId, ContractVKShare, VKShareKey<'a>>,
}

impl<'a> IndexList<ContractVKShare> for VkShareIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<ContractVKShare>> + '_> {
        let v: Vec<&dyn Index<ContractVKShare>> = vec![&self.epoch_id];
        Box::new(v.into_iter())
    }
}

pub(crate) fn vk_shares<'a>() -> IndexedMap<'a, VKShareKey<'a>, ContractVKShare, VkShareIndex<'a>> {
    let indexes = VkShareIndex {
        epoch_id: MultiIndex::new(|_pk, d| d.epoch_id, "vksp", "vkse"),
    };
    IndexedMap::new("vksp", indexes)
}

pub(crate) fn next_node_index(store: &mut dyn Storage) -> StdResult<NodeIndex> {
    // make sure we don't start from 0, otherwise all the crypto breaks (kinda)
    let id: NodeIndex = NODE_INDEX_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NODE_INDEX_COUNTER.save(store, &id)?;
    Ok(id)
}

#[cw_serde]
pub enum EmptyMessage {}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn instantiate(
    _: DepsMut<'_>,
    _: Env,
    _: MessageInfo,
    _: EmptyMessage,
) -> Result<Response, StdError> {
    Ok(Response::new())
}

/// Handle an incoming message
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn execute(
    _: DepsMut<'_>,
    _: Env,
    _: MessageInfo,
    _: EmptyMessage,
) -> Result<Response, StdError> {
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn query(_: Deps<'_>, _: Env, _: EmptyMessage) -> Result<QueryResponse, StdError> {
    Ok(Default::default())
}

// LIMITATION: we're not storing dealings themselves
#[cfg_attr(not(feature = "library"), cosmwasm_std::entry_point)]
pub fn migrate(deps: DepsMut<'_>, env: Env, msg: MigrateMsg) -> Result<Response, StdError> {
    // on migration immediately attempt to rewrite the storage
    let threshold = (2 * msg.dealers.len() as u64 + 3 - 1) / 3;
    let epoch = CURRENT_EPOCH.load(deps.storage)?;
    assert_eq!(0, epoch.epoch_id);

    // set epoch data
    THRESHOLD.save(deps.storage, &threshold)?;
    EPOCH_THRESHOLDS.save(deps.storage, 0, &threshold)?;
    let duration = epoch
        .time_configuration
        .state_duration(EpochState::InProgress);

    CURRENT_EPOCH.save(
        deps.storage,
        &Epoch {
            state: EpochState::InProgress,
            epoch_id: 0,
            state_progress: Default::default(),
            time_configuration: epoch.time_configuration,
            deadline: duration.map(|d| env.block.time.plus_seconds(d)),
        },
    )?;

    // set dealer data
    for dealer in msg.dealers {
        let node_index = next_node_index(deps.storage)?;
        DEALERS_INDICES.save(deps.storage, &dealer.owner, &node_index)?;

        let registration_details = DealerRegistrationDetails {
            bte_public_key_with_proof: "fakekey".to_string(),
            ed25519_identity: dealer.ed25519_identity,
            announce_address: dealer.announce.clone(),
        };
        let vk_share = ContractVKShare {
            share: dealer.vk,
            announce_address: dealer.announce,
            node_index,
            owner: dealer.owner.clone(),
            epoch_id: 0,
            verified: true,
        };

        EPOCH_DEALERS_MAP.save(deps.storage, (0, &dealer.owner), &registration_details)?;
        vk_shares().save(deps.storage, (&dealer.owner, 0), &vk_share)?;
    }

    Ok(Response::new())
}
