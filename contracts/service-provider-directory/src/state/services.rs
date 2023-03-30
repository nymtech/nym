use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex};
use nym_service_provider_directory_common::{NymAddress, Service, ServiceId};

use crate::{
    constants::{
        MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS, MAX_NUMBER_OF_PROVIDERS_PER_OWNER,
        SERVICE_DEFAULT_RETRIEVAL_LIMIT, SERVICE_MAX_RETRIEVAL_LIMIT,
    },
    error::{ContractError, Result},
};

const SERVICES_PK_NAMESPACE: &str = "sernames";
const SERVICES_OWNER_IDX_NAMESPACE: &str = "serown";
const SERVICES_NYM_ADDRESS_IDX_NAMESPACE: &str = "sernyma";

struct ServiceIndex<'a> {
    pub(crate) nym_address: MultiIndex<'a, String, Service, ServiceId>,
    pub(crate) owner: MultiIndex<'a, Addr, Service, ServiceId>,
}

impl<'a> IndexList<Service> for ServiceIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Service>> + '_> {
        let v: Vec<&dyn Index<Service>> = vec![&self.nym_address, &self.owner];
        Box::new(v.into_iter())
    }
}

fn services<'a>() -> IndexedMap<'a, ServiceId, Service, ServiceIndex<'a>> {
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

pub fn save(store: &mut dyn Storage, new_service: &Service) -> Result<ServiceId> {
    let service_id = super::next_service_id_counter(store)?;
    services().save(store, service_id, &new_service)?;
    Ok(service_id)
}

pub fn remove(store: &mut dyn Storage, service_id: ServiceId) -> Result<()> {
    Ok(services().remove(store, service_id)?)
}

pub fn has_service(store: &dyn Storage, service_id: ServiceId) -> bool {
    services().has(store, service_id)
}

pub fn load_id(store: &dyn Storage, service_id: ServiceId) -> Result<Service> {
    services().load(store, service_id).map_err(|err| match err {
        StdError::NotFound { .. } => ContractError::NotFound { service_id },
        err => err.into(),
    })
}

pub fn load_owner(store: &dyn Storage, owner: Addr) -> Result<Vec<(ServiceId, Service)>> {
    let services = services()
        .idx
        .owner
        .prefix(owner)
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_PROVIDERS_PER_OWNER as usize)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(services.into())
}

pub fn load_nym_address(
    store: &dyn Storage,
    nym_address: NymAddress,
) -> Result<Vec<(ServiceId, Service)>> {
    let services = services()
        .idx
        .nym_address
        .prefix(nym_address.to_string())
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_ALIASES_FOR_NYM_ADDRESS as usize)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(services.into())
}

pub fn load_all_paged(
    store: &dyn Storage,
    start_after: Option<ServiceId>,
    limit: Option<u32>,
) -> Result<(Vec<(ServiceId, Service)>, Option<ServiceId>, usize)> {
    let limit = limit
        .unwrap_or(SERVICE_DEFAULT_RETRIEVAL_LIMIT)
        .min(SERVICE_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let services = services()
        .range(store, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = services.last().map(|service| service.0);
    Ok((services, start_next_after, limit))
}

#[cfg(test)]
mod tests {
    use crate::{
        error::ContractError,
        test_helpers::{
            fixture::{service_fixture, service_fixture_by_name},
            helpers::instantiate_test_contract,
        },
    };

    use super::*;

    #[test]
    fn save_works() {
        let mut deps = instantiate_test_contract();
        assert!(!has_service(&deps.storage, 1));
        save(deps.as_mut().storage, &service_fixture()).unwrap();
        assert!(has_service(&deps.storage, 1));
    }

    #[test]
    fn save_and_check_incorrect_id_fails() {
        let mut deps = instantiate_test_contract();
        assert!(!has_service(&deps.storage, 2));
        save(deps.as_mut().storage, &service_fixture()).unwrap();
        assert!(!has_service(&deps.storage, 2));
    }

    #[test]
    fn remove_works() {
        let mut deps = instantiate_test_contract();
        let id = save(deps.as_mut().storage, &service_fixture()).unwrap();
        assert!(has_service(&deps.storage, id));
        remove(deps.as_mut().storage, id).unwrap();
        assert!(!has_service(&deps.storage, id));
    }

    #[test]
    fn load_by_id_works() {
        let mut deps = instantiate_test_contract();
        let id = save(deps.as_mut().storage, &service_fixture()).unwrap();
        let service = load_id(deps.as_ref().storage, id).unwrap();
        assert_eq!(service, service_fixture());
    }

    #[test]
    fn load_by_wrong_id_returns_not_found() {
        let mut deps = instantiate_test_contract();
        let id = save(deps.as_mut().storage, &service_fixture()).unwrap();
        assert_eq!(
            load_id(deps.as_ref().storage, id + 1).unwrap_err(),
            ContractError::NotFound { service_id: id + 1 }
        );
    }

    #[test]
    fn load_by_owner_works() {
        let mut deps = instantiate_test_contract();
        save(deps.as_mut().storage, &service_fixture_by_name("a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("c")).unwrap();
        assert_eq!(
            load_owner(&deps.storage, Addr::unchecked("steve")).unwrap(),
            vec![
                (1, service_fixture_by_name("a")),
                (2, service_fixture_by_name("b")),
                (3, service_fixture_by_name("c")),
            ]
        );
    }

    #[test]
    fn load_by_wrong_owner_returns_empty() {
        let mut deps = instantiate_test_contract();
        save(deps.as_mut().storage, &service_fixture_by_name("a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("c")).unwrap();
        assert_eq!(
            load_owner(&deps.storage, Addr::unchecked("timmy")).unwrap(),
            vec![]
        );
    }

    #[test]
    fn load_by_nym_address_works() {
        let mut deps = instantiate_test_contract();
        save(deps.as_mut().storage, &service_fixture_by_name("a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("c")).unwrap();
        assert_eq!(
            load_nym_address(&deps.storage, NymAddress::new("b")).unwrap(),
            vec![(2, service_fixture_by_name("b"))]
        );
    }

    #[test]
    fn load_by_wrong_nym_address_returns_empty() {
        let mut deps = instantiate_test_contract();
        save(deps.as_mut().storage, &service_fixture_by_name("a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("c")).unwrap();
        assert_eq!(
            load_nym_address(&deps.storage, NymAddress::new("d")).unwrap(),
            vec![]
        );
    }

    #[test]
    fn load_all_paged_with_no_limit_works() {
        let mut deps = instantiate_test_contract();
        save(deps.as_mut().storage, &service_fixture_by_name("a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("b")).unwrap();
        assert_eq!(
            load_all_paged(&deps.storage, None, None).unwrap(),
            (
                vec![
                    (1, service_fixture_by_name("a")),
                    (2, service_fixture_by_name("b"))
                ],
                Some(2),
                100,
            )
        );
    }

    #[test]
    fn load_all_paged_with_limit_works() {
        let mut deps = instantiate_test_contract();
        save(deps.as_mut().storage, &service_fixture_by_name("a")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("b")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("c")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("d")).unwrap();
        save(deps.as_mut().storage, &service_fixture_by_name("e")).unwrap();
        assert_eq!(
            load_all_paged(&deps.storage, None, Some(2)).unwrap(),
            (
                vec![
                    (1, service_fixture_by_name("a")),
                    (2, service_fixture_by_name("b"))
                ],
                Some(2),
                2,
            )
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), Some(2)).unwrap(),
            (
                vec![
                    (3, service_fixture_by_name("c")),
                    (4, service_fixture_by_name("d"))
                ],
                Some(4),
                2,
            )
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(4), Some(2)).unwrap(),
            (vec![(5, service_fixture_by_name("e")),], Some(5), 2,)
        );
    }

    #[test]
    #[ignore]
    fn max_page_limit_is_applied() {
        todo!();
    }
}
