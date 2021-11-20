use cosmwasm_std::Storage;
use cosmwasm_storage::bucket;
use cosmwasm_storage::bucket_read;
use cosmwasm_storage::Bucket;
use cosmwasm_storage::ReadonlyBucket;
use mixnet_contract::GatewayBond;
use mixnet_contract::IdentityKey;

// storage prefixes
const PREFIX_GATEWAYS: &[u8] = b"gt";
const PREFIX_GATEWAYS_OWNERS: &[u8] = b"go";

pub fn gateways(storage: &mut dyn Storage) -> Bucket<GatewayBond> {
    bucket(storage, PREFIX_GATEWAYS)
}

pub fn gateways_read(storage: &dyn Storage) -> ReadonlyBucket<GatewayBond> {
    bucket_read(storage, PREFIX_GATEWAYS)
}

// owner address -> node identity
pub fn gateways_owners(storage: &mut dyn Storage) -> Bucket<IdentityKey> {
    bucket(storage, PREFIX_GATEWAYS_OWNERS)
}

pub fn gateways_owners_read(storage: &dyn Storage) -> ReadonlyBucket<IdentityKey> {
    bucket_read(storage, PREFIX_GATEWAYS_OWNERS)
}

// currently not used outside tests
#[cfg(test)]
mod tests {
    use cosmwasm_std::StdResult;
    use cosmwasm_std::Storage;

    use super::super::storage;
    use crate::support::tests::test_helpers;
    use config::defaults::DENOM;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{coin, Addr, Uint128};
    use mixnet_contract::Gateway;
    use mixnet_contract::GatewayBond;
    use mixnet_contract::IdentityKey;

    // currently this is only used in tests but may become useful later on
    pub(crate) fn read_gateway_bond(
        storage: &dyn Storage,
        identity: &[u8],
    ) -> StdResult<cosmwasm_std::Uint128> {
        let bucket = storage::gateways_read(storage);
        let node = bucket.load(identity)?;
        Ok(node.bond_amount.amount)
    }

    #[test]
    fn gateway_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let bond1 = test_helpers::gateway_bond_fixture();
        let bond2 = test_helpers::gateway_bond_fixture();
        storage::gateways(&mut storage)
            .save(b"bond1", &bond1)
            .unwrap();
        storage::gateways(&mut storage)
            .save(b"bond2", &bond2)
            .unwrap();

        let res1 = storage::gateways_read(&storage).load(b"bond1").unwrap();
        let res2 = storage::gateways_read(&storage).load(b"bond2").unwrap();
        assert_eq!(bond1, res1);
        assert_eq!(bond2, res2);
    }

    #[test]
    fn reading_gateway_bond() {
        let mut mock_storage = MockStorage::new();
        let node_owner: Addr = Addr::unchecked("node-owner");
        let node_identity: IdentityKey = "nodeidentity".into();

        // produces an error if target gateway doesn't exist
        let res = read_gateway_bond(&mock_storage, node_owner.as_bytes());
        assert!(res.is_err());

        // returns appropriate value otherwise
        let bond_value = 1000;

        let gateway_bond = GatewayBond {
            bond_amount: coin(bond_value, DENOM),
            owner: node_owner.clone(),
            block_height: 12_345,
            gateway: Gateway {
                identity_key: node_identity.clone(),
                ..test_helpers::gateway_fixture()
            },
        };

        storage::gateways(&mut mock_storage)
            .save(node_identity.as_bytes(), &gateway_bond)
            .unwrap();

        assert_eq!(
            Uint128(bond_value),
            read_gateway_bond(&mock_storage, node_identity.as_bytes()).unwrap()
        );
    }
}
