// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{VK_SHARES_EPOCH_ID_IDX_NAMESPACE, VK_SHARES_PK_NAMESPACE};
use crate::epoch_state::storage::CURRENT_EPOCH;
use crate::error::ContractError;
use cosmwasm_std::{Addr, Order, StdResult, Storage};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::verification_key::ContractVKShare;
use std::collections::HashMap;

pub(crate) const VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const VERIFICATION_KEY_SHARES_PAGE_DEFAULT_LIMIT: u32 = 50;

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
        epoch_id: MultiIndex::new(
            |_pk, d| d.epoch_id,
            VK_SHARES_PK_NAMESPACE,
            VK_SHARES_EPOCH_ID_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(VK_SHARES_PK_NAMESPACE, indexes)
}

// TODO: this is a ticking time bomb and will cause us headache when we start running out of gas... again...
// not sure how to fix it yet...
// maybe by completely removing it by relying on cw4 hooks?
#[deprecated]
pub(crate) fn verified_dealers(storage: &dyn Storage) -> Result<Vec<Addr>, ContractError> {
    let epoch_id = CURRENT_EPOCH.load(storage)?.epoch_id;
    Ok(vk_shares()
        .idx
        .epoch_id
        .prefix(epoch_id)
        .range(storage, None, None, Order::Ascending)
        .flatten()
        .filter_map(|(_, share)| {
            if share.verified {
                Some(share.owner)
            } else {
                None
            }
        })
        .collect())
}

// TODO: this is a ticking time bomb and will cause us headache when we start running out of gas... again...
// not sure how to fix it yet...
// maybe by completely removing it by relying on cw4 hooks?
pub(crate) fn dealers(storage: &dyn Storage) -> Result<HashMap<Addr, bool>, ContractError> {
    let epoch_id = CURRENT_EPOCH.load(storage)?.epoch_id;

    Ok(vk_shares()
        .idx
        .epoch_id
        .prefix(epoch_id)
        .range(storage, None, None, Order::Ascending)
        .map(|maybe_share| maybe_share.map(|(_, v)| (v.owner, v.verified)))
        .collect::<StdResult<_>>()?)
}
