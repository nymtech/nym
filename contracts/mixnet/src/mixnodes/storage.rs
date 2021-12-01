// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use config::defaults::DENOM;
use cosmwasm_std::{StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Map, UniqueIndex};
use mixnet_contract::{Addr, Coin, IdentityKeyRef, Layer, MixNode, MixNodeBond};
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
    pub bond_amount: Coin,
    pub owner: Addr,
    pub layer: Layer,
    pub block_height: u64,
    pub mix_node: MixNode,
    pub profit_margin_percent: Option<u8>,
    pub proxy: Option<Addr>,
}

impl StoredMixnodeBond {
    pub(crate) fn new(
        bond_amount: Coin,
        owner: Addr,
        layer: Layer,
        block_height: u64,
        mix_node: MixNode,
        profit_margin_percent: Option<u8>,
        proxy: Option<Addr>,
    ) -> Self {
        StoredMixnodeBond {
            bond_amount,
            owner,
            layer,
            block_height,
            mix_node,
            profit_margin_percent,
            proxy,
        }
    }

    pub(crate) fn attach_delegation(self, total_delegation: Uint128) -> MixNodeBond {
        MixNodeBond {
            total_delegation: Coin {
                denom: self.bond_amount.denom.clone(),
                amount: total_delegation,
            },
            bond_amount: self.bond_amount,
            owner: self.owner,
            layer: self.layer,
            block_height: self.block_height,
            mix_node: self.mix_node,
            profit_margin_percent: self.profit_margin_percent,
            proxy: self.proxy,
        }
    }

    pub(crate) fn identity(&self) -> &String {
        &self.mix_node.identity_key
    }

    pub(crate) fn bond_amount(&self) -> Coin {
        self.bond_amount.clone()
    }
}

impl Display for StoredMixnodeBond {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "amount: {}, owner: {}, identity: {}",
            self.bond_amount, self.owner, self.mix_node.identity_key
        )
    }
}

pub(crate) fn read_mixnode_bond(
    storage: &dyn Storage,
    mix_identity: IdentityKeyRef,
) -> StdResult<Option<MixNodeBond>> {
    let stored_bond = mixnodes().may_load(storage, mix_identity)?;
    match stored_bond {
        None => Ok(None),
        Some(stored_bond) => {
            let total_delegation = TOTAL_DELEGATION.may_load(storage, mix_identity)?;
            Ok(Some(MixNodeBond {
                bond_amount: stored_bond.bond_amount,
                total_delegation: Coin {
                    denom: DENOM.to_owned(),
                    amount: total_delegation.unwrap_or_default(),
                },
                owner: stored_bond.owner,
                layer: stored_bond.layer,
                block_height: stored_bond.block_height,
                mix_node: stored_bond.mix_node,
                profit_margin_percent: stored_bond.profit_margin_percent,
                proxy: stored_bond.proxy,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::storage;
    use super::*;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::IdentityKey;
    use mixnet_contract::MixNode;

    #[test]
    fn mixnode_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = test_helpers::stored_mixnode_bond_fixture("owner1");
        let bond2 = test_helpers::stored_mixnode_bond_fixture("owner2");
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
        let res = storage::read_mixnode_bond(&mock_storage, &node_identity).unwrap();
        assert!(res.is_none());

        // returns appropriate value otherwise
        let bond_value = 1000000000;

        let mixnode_bond = StoredMixnodeBond {
            bond_amount: coin(bond_value, DENOM),
            owner: node_owner.clone(),
            layer: Layer::One,
            block_height: 12_345,
            mix_node: MixNode {
                identity_key: node_identity.clone(),
                ..test_helpers::mix_node_fixture()
            },
            profit_margin_percent: None,
            proxy: None,
        };

        storage::mixnodes()
            .save(&mut mock_storage, &node_identity, &mixnode_bond)
            .unwrap();

        assert_eq!(
            Uint128::new(bond_value),
            storage::read_mixnode_bond(&mock_storage, node_identity.as_str())
                .unwrap()
                .unwrap()
                .bond_amount
                .amount
        );
    }
}
