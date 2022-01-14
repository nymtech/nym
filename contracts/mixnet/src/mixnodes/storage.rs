// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::DENOM;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, UniqueIndex};
use mixnet_contract_common::{Addr, Coin, IdentityKeyRef, Layer, MixNode, MixNodeBond};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

// storage prefixes
const TOTAL_DELEGATION_NAMESPACE: &str = "td";
const MIXNODES_PK_NAMESPACE: &str = "mn";
const MIXNODES_OWNER_IDX_NAMESPACE: &str = "mno";

// paged retrieval limits for all queries and transactions
pub(crate) const BOND_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

pub(crate) const TOTAL_DELEGATION: Map<IdentityKeyRef, Uint128> =
    Map::new(TOTAL_DELEGATION_NAMESPACE);

pub(crate) struct MixnodeBondIndex<'a> {
    pub(crate) owner: UniqueIndex<'a, Addr, StoredMixnodeBond>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<StoredMixnodeBond> for MixnodeBondIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<StoredMixnodeBond>> + '_> {
        let v: Vec<&dyn Index<StoredMixnodeBond>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

// mixnodes() is the storage access function.
pub(crate) fn mixnodes<'a>(
) -> IndexedMap<'a, IdentityKeyRef<'a>, StoredMixnodeBond, MixnodeBondIndex<'a>> {
    let indexes = MixnodeBondIndex {
        owner: UniqueIndex::new(|d| d.owner.clone(), MIXNODES_OWNER_IDX_NAMESPACE),
    };
    IndexedMap::new(MIXNODES_PK_NAMESPACE, indexes)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub(crate) struct StoredMixnodeBond {
    pub pledge_amount: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub block_height: u64,
    pub mix_node: MixNode,
    pub proxy: Option<Addr>,
}

impl StoredMixnodeBond {
    pub(crate) fn new(
        pledge_amount: Coin,
        owner: Addr,
        layer: Layer,
        block_height: u64,
        mix_node: MixNode,
        proxy: Option<Addr>,
    ) -> Self {
        StoredMixnodeBond {
            pledge_amount,
            owner,
            layer,
            block_height,
            mix_node,
            proxy,
        }
    }

    pub(crate) fn attach_delegation(self, total_delegation: Uint128) -> MixNodeBond {
        MixNodeBond {
            total_delegation: Coin {
                denom: self.pledge_amount.denom.clone(),
                amount: total_delegation,
            },
            pledge_amount: self.pledge_amount,
            owner: self.owner,
            layer: self.layer,
            block_height: self.block_height,
            mix_node: self.mix_node,
            proxy: self.proxy,
        }
    }

    pub(crate) fn identity(&self) -> &String {
        &self.mix_node.identity_key
    }

    pub(crate) fn pledge_amount(&self) -> Coin {
        self.pledge_amount.clone()
    }
}

impl Display for StoredMixnodeBond {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "amount: {}, owner: {}, identity: {}",
            self.pledge_amount, self.owner, self.mix_node.identity_key
        )
    }
}

pub(crate) fn read_full_mixnode_bond(
    storage: &dyn Storage,
    mix_identity: IdentityKeyRef,
) -> StdResult<Option<MixNodeBond>> {
    let stored_bond = mixnodes().may_load(storage, mix_identity)?;
    match stored_bond {
        None => Ok(None),
        Some(stored_bond) => {
            let total_delegation = TOTAL_DELEGATION.may_load(storage, mix_identity)?;
            Ok(Some(MixNodeBond {
                pledge_amount: stored_bond.pledge_amount,
                total_delegation: Coin {
                    denom: DENOM.to_owned(),
                    amount: total_delegation.unwrap_or_default(),
                },
                owner: stored_bond.owner,
                layer: stored_bond.layer,
                block_height: stored_bond.block_height,
                mix_node: stored_bond.mix_node,
                proxy: stored_bond.proxy,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::storage;
    use super::*;
    use crate::support::tests;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract_common::{IdentityKey, MixNode};

    #[test]
    fn mixnode_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = tests::fixtures::stored_mixnode_bond_fixture("owner1");
        let bond2 = tests::fixtures::stored_mixnode_bond_fixture("owner2");
        mixnodes().save(&mut storage, "bond1", &bond1).unwrap();
        mixnodes().save(&mut storage, "bond2", &bond2).unwrap();

        let res1 = mixnodes().load(&storage, "bond1").unwrap();
        let res2 = mixnodes().load(&storage, "bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn reading_mixnode_bond() {
        let mut mock_storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces a None if target mixnode doesn't exist
        let res = storage::read_full_mixnode_bond(&mock_storage, &node_identity).unwrap();
        assert!(res.is_none());

        // returns appropriate value otherwise
        let pledge_value = 1000000000;

        let mixnode_bond = StoredMixnodeBond {
            pledge_amount: coin(pledge_value, DENOM),
            owner: node_owner,
            layer: Layer::One,
            block_height: 12_345,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..tests::fixtures::mix_node_fixture()
            },
            proxy: None,
        };

        storage::mixnodes()
            .save(&mut mock_storage, &node_identity, &mixnode_bond)
            .unwrap();

        assert_eq!(
            Uint128::new(pledge_value),
            storage::read_full_mixnode_bond(&mock_storage, node_identity.as_str())
                .unwrap()
                .unwrap()
                .pledge_amount
                .amount
        );
    }
}
