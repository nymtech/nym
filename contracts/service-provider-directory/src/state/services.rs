use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use nym_service_provider_directory_common::{Service, ServiceId};

const SERVICES_PK_NAMESPACE: &str = "sernames";
const SERVICES_OWNER_IDX_NAMESPACE: &str = "serown";
const SERVICES_NYM_ADDRESS_IDX_NAMESPACE: &str = "sernyma";

pub(crate) struct ServiceIndex<'a> {
    pub(crate) nym_address: MultiIndex<'a, String, Service, ServiceId>,
    pub(crate) owner: MultiIndex<'a, Addr, Service, ServiceId>,
}

impl<'a> IndexList<Service> for ServiceIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Service>> + '_> {
        let v: Vec<&dyn Index<Service>> = vec![&self.nym_address, &self.owner];
        Box::new(v.into_iter())
    }
}

pub(crate) fn services<'a>() -> IndexedMap<'a, ServiceId, Service, ServiceIndex<'a>> {
    let indexes = ServiceIndex {
        nym_address: MultiIndex::new(
            |d| d.nym_address.to_string(),
            SERVICES_PK_NAMESPACE,
            SERVICES_NYM_ADDRESS_IDX_NAMESPACE,
        ),
        owner: MultiIndex::new(
            |d| d.owner.clone(),
            SERVICES_PK_NAMESPACE,
            SERVICES_OWNER_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(SERVICES_PK_NAMESPACE, indexes)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{Order, StdError, StdResult};

    use crate::test_helpers::{
        fixture::{service_fixture, service_fixture_by_name},
        helpers::{announce_service, instantiate_test_contract},
    };

    use super::*;

    #[test]
    fn save_and_load_returns_keys() {
        let mut deps = instantiate_test_contract();

        announce_service(deps.as_mut(), service_fixture());
        announce_service(deps.as_mut(), service_fixture());
        announce_service(deps.as_mut(), service_fixture());

        let keys = services().keys(&deps.storage, None, None, Order::Ascending);
        assert_eq!(keys.count(), 3);
    }

    #[test]
    fn save_and_load_by_id_works() {
        let mut deps = instantiate_test_contract();

        announce_service(deps.as_mut(), service_fixture_by_name("a"));
        announce_service(deps.as_mut(), service_fixture_by_name("b"));
        announce_service(deps.as_mut(), service_fixture_by_name("c"));
        // Load not "in-order"
        assert_eq!(
            services().load(&deps.storage, 1).unwrap(),
            service_fixture_by_name("a")
        );
        assert_eq!(
            services().load(&deps.storage, 3).unwrap(),
            service_fixture_by_name("c")
        );
        assert_eq!(
            services().load(&deps.storage, 2).unwrap(),
            service_fixture_by_name("b")
        );
    }

    #[test]
    fn save_and_load_by_wrong_id_fails() {
        let mut deps = instantiate_test_contract();

        announce_service(deps.as_mut(), service_fixture_by_name("a"));
        announce_service(deps.as_mut(), service_fixture_by_name("b"));
        announce_service(deps.as_mut(), service_fixture_by_name("c"));
        assert!(matches!(
            services().load(&deps.storage, 4).unwrap_err(),
            StdError::NotFound { .. }
        ));
    }

    #[test]
    fn save_and_load_by_owner_works() {
        let mut deps = instantiate_test_contract();

        announce_service(deps.as_mut(), service_fixture_by_name("a"));
        announce_service(deps.as_mut(), service_fixture_by_name("b"));
        announce_service(deps.as_mut(), service_fixture_by_name("c"));

        let services = services()
            .idx
            .owner
            .prefix(Addr::unchecked("steve"))
            .range(&deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();
        assert_eq!(
            services,
            vec![
                (1, service_fixture_by_name("a")),
                (2, service_fixture_by_name("b")),
                (3, service_fixture_by_name("c")),
            ]
        );
    }
}
