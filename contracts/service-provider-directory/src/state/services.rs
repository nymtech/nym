use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};
use nym_service_provider_directory_common::{NymAddress, Service, ServiceId};

use crate::{
    constants::{
        MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS, MAX_NUMBER_OF_PROVIDERS_PER_ANNOUNCER,
        SERVICES_ANNOUNCER_IDX_NAMESPACE, SERVICES_NYM_ADDRESS_IDX_NAMESPACE,
        SERVICES_PK_NAMESPACE, SERVICE_DEFAULT_RETRIEVAL_LIMIT, SERVICE_MAX_RETRIEVAL_LIMIT,
    },
    Result, SpContractError,
};

struct ServiceIndex<'a> {
    pub(crate) nym_address: MultiIndex<'a, String, Service, ServiceId>,
    pub(crate) announcer: MultiIndex<'a, Addr, Service, ServiceId>,
}

impl<'a> IndexList<Service> for ServiceIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Service>> + '_> {
        let v: Vec<&dyn Index<Service>> = vec![&self.nym_address, &self.announcer];
        Box::new(v.into_iter())
    }
}

fn services<'a>() -> IndexedMap<'a, ServiceId, Service, ServiceIndex<'a>> {
    let indexes = ServiceIndex {
        nym_address: MultiIndex::new(
            |_pk, d| d.service.nym_address.to_string(),
            SERVICES_PK_NAMESPACE,
            SERVICES_NYM_ADDRESS_IDX_NAMESPACE,
        ),
        announcer: MultiIndex::new(
            |_pk, d| d.announcer.clone(),
            SERVICES_PK_NAMESPACE,
            SERVICES_ANNOUNCER_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(SERVICES_PK_NAMESPACE, indexes)
}

pub fn save(store: &mut dyn Storage, new_service: &Service) -> Result<()> {
    let service_id = new_service.service_id;
    services().save(store, service_id, new_service)?;
    Ok(())
}

pub fn remove(store: &mut dyn Storage, service_id: ServiceId) -> Result<()> {
    Ok(services().remove(store, service_id)?)
}

pub fn has_service(store: &dyn Storage, service_id: ServiceId) -> bool {
    services().has(store, service_id)
}

pub fn load_id(store: &dyn Storage, service_id: ServiceId) -> Result<Service> {
    services().load(store, service_id).map_err(|err| match err {
        StdError::NotFound { .. } => SpContractError::NotFound { service_id },
        err => err.into(),
    })
}

pub fn load_announcer(store: &dyn Storage, announcer: Addr) -> Result<Vec<Service>> {
    let services = services()
        .idx
        .announcer
        .prefix(announcer)
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_PROVIDERS_PER_ANNOUNCER as usize)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(services)
}

pub fn load_nym_address(store: &dyn Storage, nym_address: NymAddress) -> Result<Vec<Service>> {
    let services = services()
        .idx
        .nym_address
        .prefix(nym_address.to_string())
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS as usize)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(services)
}

#[derive(Debug, PartialEq)]
pub struct PagedLoad {
    pub services: Vec<Service>,
    pub limit: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<ServiceId>,
}

pub fn load_all_paged(
    store: &dyn Storage,
    limit: Option<u32>,
    start_after: Option<ServiceId>,
) -> Result<PagedLoad> {
    let limit = limit
        .unwrap_or(SERVICE_DEFAULT_RETRIEVAL_LIMIT)
        .min(SERVICE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let services = services()
        .range(store, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|item| item.1))
        .collect::<StdResult<Vec<Service>>>()?;

    let start_next_after = services.last().map(|service| service.service_id);

    Ok(PagedLoad {
        services,
        limit,
        start_next_after,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::{
        testing::{MockApi, MockQuerier},
        MemoryStorage, OwnedDeps,
    };
    use rstest::rstest;

    use crate::test_helpers::{
        fixture::{service_fixture, service_fixture_with_address},
        transactions::instantiate_test_contract,
    };

    use super::*;

    type TestDeps = OwnedDeps<MemoryStorage, MockApi, MockQuerier>;

    #[rstest::fixture]
    fn deps() -> TestDeps {
        instantiate_test_contract()
    }

    #[rstest]
    fn save_works(mut deps: TestDeps) {
        assert!(!has_service(&deps.storage, 1));
        save(deps.as_mut().storage, &service_fixture(1)).unwrap();
        assert!(has_service(&deps.storage, 1));
    }

    #[rstest]
    fn save_and_check_incorrect_id_fails(mut deps: TestDeps) {
        assert!(!has_service(&deps.storage, 2));
        save(deps.as_mut().storage, &service_fixture(1)).unwrap();
        assert!(!has_service(&deps.storage, 2));
    }

    #[rstest]
    fn remove_works(mut deps: TestDeps) {
        let id = 1;
        save(deps.as_mut().storage, &service_fixture(id)).unwrap();
        assert!(has_service(&deps.storage, id));
        remove(deps.as_mut().storage, id).unwrap();
        assert!(!has_service(&deps.storage, id));
    }

    #[rstest]
    fn load_by_id_works(mut deps: TestDeps) {
        let id = 1;
        save(deps.as_mut().storage, &service_fixture(id)).unwrap();
        let service = load_id(deps.as_ref().storage, id).unwrap();
        assert_eq!(service, service_fixture(id));
    }

    #[rstest]
    fn load_by_wrong_id_returns_not_found(mut deps: TestDeps) {
        let id = 1;
        save(deps.as_mut().storage, &service_fixture(id)).unwrap();
        assert_eq!(
            load_id(deps.as_ref().storage, id + 1).unwrap_err(),
            SpContractError::NotFound { service_id: id + 1 }
        );
    }

    #[rstest]
    fn load_by_announcer_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &service_fixture_with_address(1, "a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(2, "b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(3, "c")).unwrap();
        assert_eq!(
            load_announcer(&deps.storage, Addr::unchecked("steve")).unwrap(),
            vec![
                service_fixture_with_address(1, "a"),
                service_fixture_with_address(2, "b"),
                service_fixture_with_address(3, "c"),
            ]
        );
    }

    #[rstest]
    fn load_by_wrong_announcer_returns_empty(mut deps: TestDeps) {
        save(deps.as_mut().storage, &service_fixture_with_address(1, "a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(2, "b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(3, "c")).unwrap();
        assert_eq!(
            load_announcer(&deps.storage, Addr::unchecked("timmy")).unwrap(),
            vec![]
        );
    }

    #[rstest]
    fn load_by_nym_address_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &service_fixture_with_address(1, "a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(2, "b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(3, "c")).unwrap();
        assert_eq!(
            load_nym_address(&deps.storage, NymAddress::new("b")).unwrap(),
            vec![service_fixture_with_address(2, "b")]
        );
    }

    #[rstest]
    fn load_by_wrong_nym_address_returns_empty(mut deps: TestDeps) {
        save(deps.as_mut().storage, &service_fixture_with_address(1, "a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(2, "b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(3, "c")).unwrap();
        assert_eq!(
            load_nym_address(&deps.storage, NymAddress::new("d")).unwrap(),
            vec![]
        );
    }

    #[rstest]
    fn load_all_paged_with_no_limit_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &service_fixture_with_address(1, "a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(2, "b")).unwrap();
        assert_eq!(
            load_all_paged(&deps.storage, None, None).unwrap(),
            PagedLoad {
                services: vec![
                    service_fixture_with_address(1, "a"),
                    service_fixture_with_address(2, "b")
                ],
                start_next_after: Some(2),
                limit: 100,
            }
        );
    }

    #[rstest]
    fn load_all_paged_with_limit_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &service_fixture_with_address(1, "a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(2, "b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(3, "c")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(4, "d")).unwrap();
        save(deps.as_mut().storage, &service_fixture_with_address(5, "e")).unwrap();
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), None).unwrap(),
            PagedLoad {
                services: vec![
                    service_fixture_with_address(1, "a"),
                    service_fixture_with_address(2, "b")
                ],
                limit: 2,
                start_next_after: Some(2),
            }
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), Some(2)).unwrap(),
            PagedLoad {
                services: vec![
                    service_fixture_with_address(3, "c"),
                    service_fixture_with_address(4, "d")
                ],
                limit: 2,
                start_next_after: Some(4),
            }
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), Some(4)).unwrap(),
            PagedLoad {
                services: vec![service_fixture_with_address(5, "e")],
                start_next_after: Some(5),
                limit: 2,
            }
        );
    }

    #[test]
    #[ignore]
    fn max_page_limit_is_applied() {
        todo!();
    }
}
