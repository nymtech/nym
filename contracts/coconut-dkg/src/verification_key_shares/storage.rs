// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::constants::{VK_SHARES_EPOCH_ID_IDX_NAMESPACE, VK_SHARES_PK_NAMESPACE};
use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_dkg_common::verification_key::ContractVKShare;

pub(crate) const VERIFICATION_KEY_SHARES_PAGE_MAX_LIMIT: u32 = 30;
pub(crate) const VERIFICATION_KEY_SHARES_PAGE_DEFAULT_LIMIT: u32 = 10;

type VKShareKey<'a> = (&'a Addr, EpochId);

pub(crate) struct VkShareIndex<'a> {
    pub(crate) epoch_id: MultiIndex<'a, EpochId, ContractVKShare, VKShareKey<'a>>,
}

impl IndexList<ContractVKShare> for VkShareIndex<'_> {
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
