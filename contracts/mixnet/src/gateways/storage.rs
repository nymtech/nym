// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, UniqueIndex};
use mixnet_contract::{GatewayBond, IdentityKeyRef};

// storage prefixes
const GATEWAYS_PK_NAMESPACE: &str = "gt";
const GATEWAYS_OWNER_IDX_NAMESPACE: &str = "gto";

pub(crate) struct GatewayBondIndex<'a> {
    pub(crate) owner: UniqueIndex<'a, Addr, GatewayBond>,
}

// IndexList is just boilerplate code for fetching a struct's indexes
// note that from my understanding this will be converted into a macro at some point in the future
impl<'a> IndexList<GatewayBond> for GatewayBondIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<GatewayBond>> + '_> {
        let v: Vec<&dyn Index<GatewayBond>> = vec![&self.owner];
        Box::new(v.into_iter())
    }
}

// gateways() is the storage access function.
pub(crate) fn gateways<'a>() -> IndexedMap<'a, IdentityKeyRef<'a>, GatewayBond, GatewayBondIndex<'a>>
{
    let indexes = GatewayBondIndex {
        owner: UniqueIndex::new(|d| d.owner.clone(), GATEWAYS_OWNER_IDX_NAMESPACE),
    };
    IndexedMap::new(GATEWAYS_PK_NAMESPACE, indexes)
}

// currently not used outside tests
#[cfg(test)]
mod tests {
    use super::super::storage;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::StdResult;
    use cosmwasm_std::Storage;
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::GatewayBond;
    use mixnet_contract::IdentityKey;
    use mixnet_contract::{Gateway, IdentityKeyRef};

    // currently this is only used in tests but may become useful later on
    pub(crate) fn read_gateway_pledge_amount(
        storage: &dyn Storage,
        identity: IdentityKeyRef,
    ) -> StdResult<cosmwasm_std::Uint128> {
        let node = storage::gateways().load(storage, identity)?;
        Ok(node.pledge_amount.amount)
    }

    #[test]
    fn gateway_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = test_helpers::gateway_bond_fixture("owner1");
        let bond2 = test_helpers::gateway_bond_fixture("owner2");
        storage::gateways()
            .save(&mut storage, "bond1", &bond1)
            .unwrap();
        storage::gateways()
            .save(&mut storage, "bond2", &bond2)
            .unwrap();

        let res1 = storage::gateways().load(&storage, "bond1").unwrap();
        let res2 = storage::gateways().load(&storage, "bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn reading_gateway_bond() {
        let mut mock_storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces an error if target gateway doesn't exist
        let res = read_gateway_pledge_amount(&mock_storage, &node_identity);
        assert!(res.is_err());

        // returns appropriate value otherwise
        let pledge_amount = 1000;

        let gateway_bond = GatewayBond {
            pledge_amount: coin(pledge_amount, DENOM),
            owner: node_owner.clone(),
            block_height: 12_345,
            gateway: Gateway {
                identity_key: node_identity.clone(),
                ..test_helpers::gateway_fixture()
            },
            proxy: None,
        };

        storage::gateways()
            .save(&mut mock_storage, &node_identity, &gateway_bond)
            .unwrap();

        assert_eq!(
            Uint128::new(pledge_amount),
            read_gateway_pledge_amount(&mock_storage, &node_identity).unwrap()
        );
    }
}
