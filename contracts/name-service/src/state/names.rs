use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex, UniqueIndex};
use nym_name_service_common::{Address, NameId, NymName, RegisteredName};

use crate::{
    constants::{
        MAX_NUMBER_OF_NAMES_FOR_ADDRESS, MAX_NUMBER_OF_NAMES_PER_OWNER,
        NAMES_ADDRESS_IDX_NAMESPACE, NAMES_NAME_IDX_NAMESPACE, NAMES_OWNER_IDX_NAMESPACE,
        NAMES_PK_NAMESPACE, NAME_DEFAULT_RETRIEVAL_LIMIT, NAME_MAX_RETRIEVAL_LIMIT,
    },
    NameServiceError, Result,
};

struct NameIndex<'a> {
    // A name can only point to a single address
    pub(crate) name: UniqueIndex<'a, String, RegisteredName, NameId>,
    // An addresses can be pointed to by multiple names.
    pub(crate) address: MultiIndex<'a, String, RegisteredName, NameId>,
    pub(crate) owner: MultiIndex<'a, Addr, RegisteredName, NameId>,
}

impl<'a> IndexList<RegisteredName> for NameIndex<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<RegisteredName>> + '_> {
        let v: Vec<&dyn Index<RegisteredName>> = vec![&self.name, &self.address, &self.owner];
        Box::new(v.into_iter())
    }
}

fn names<'a>() -> IndexedMap<'a, NameId, RegisteredName, NameIndex<'a>> {
    let indexes = NameIndex {
        name: UniqueIndex::new(|d| d.name.name.to_string(), NAMES_NAME_IDX_NAMESPACE),
        address: MultiIndex::new(
            |_pk, d| d.name.address.to_string(),
            NAMES_PK_NAMESPACE,
            NAMES_ADDRESS_IDX_NAMESPACE,
        ),
        owner: MultiIndex::new(
            |_pk, d| d.owner.clone(),
            NAMES_PK_NAMESPACE,
            NAMES_OWNER_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(NAMES_PK_NAMESPACE, indexes)
}

pub fn save(store: &mut dyn Storage, new_name: &RegisteredName) -> Result<()> {
    let name_id = new_name.id;
    names().save(store, name_id, new_name)?;
    Ok(())
}

#[cfg(test)]
pub fn save_all(state: &mut dyn Storage, names: &[RegisteredName]) -> Result<()> {
    for name in names {
        save(state, name)?;
    }
    Ok(())
}

pub fn remove_id(store: &mut dyn Storage, name_id: NameId) -> Result<()> {
    Ok(names().remove(store, name_id)?)
}

#[cfg(test)]
pub fn remove_name(store: &mut dyn Storage, name: NymName) -> Result<NameId> {
    let registered_name = load_name(store, &name)?;
    remove_id(store, registered_name.id)?;
    Ok(registered_name.id)
}

pub fn has_name_id(store: &dyn Storage, name_id: NameId) -> bool {
    names().has(store, name_id)
}

pub fn has_name(store: &dyn Storage, name: &NymName) -> bool {
    load_name(store, name).is_ok()
}

pub fn load_id(store: &dyn Storage, name_id: NameId) -> Result<RegisteredName> {
    names().load(store, name_id).map_err(|err| match err {
        StdError::NotFound { .. } => NameServiceError::NotFound { name_id },
        err => err.into(),
    })
}

pub fn load_name(store: &dyn Storage, name: &NymName) -> Result<RegisteredName> {
    names()
        .idx
        .name
        .item(store, name.to_string())?
        .map(|record| record.1)
        .ok_or(NameServiceError::NameNotFound { name: name.clone() })
}

pub fn load_address(store: &dyn Storage, address: &Address) -> Result<Vec<RegisteredName>> {
    names()
        .idx
        .address
        .prefix(address.to_string())
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_NAMES_FOR_ADDRESS as usize)
        .map(|res| res.map(|(_, name)| name))
        .collect::<StdResult<Vec<_>>>()
        .map_err(NameServiceError::from)
}

pub fn load_owner(store: &dyn Storage, owner: Addr) -> Result<Vec<RegisteredName>> {
    names()
        .idx
        .owner
        .prefix(owner)
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_NAMES_PER_OWNER as usize)
        .map(|res| res.map(|(_, name)| name))
        .collect::<StdResult<Vec<_>>>()
        .map_err(NameServiceError::from)
}

#[derive(Debug, PartialEq)]
pub struct PagedLoad {
    pub names: Vec<RegisteredName>,
    pub limit: usize,

    /// Field indicating paging information for the following queries if the caller wishes to get further entries.
    pub start_next_after: Option<NameId>,
}

pub fn load_all_paged(
    store: &dyn Storage,
    limit: Option<u32>,
    start_after: Option<NameId>,
) -> Result<PagedLoad> {
    let limit = limit
        .unwrap_or(NAME_DEFAULT_RETRIEVAL_LIMIT)
        .min(NAME_MAX_RETRIEVAL_LIMIT) as usize;

    let start = start_after.map(Bound::exclusive);

    let names = names()
        .range(store, start, None, Order::Ascending)
        .take(limit)
        .map(|res| res.map(|(_, name)| name))
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = names.last().map(|name| name.id);

    Ok(PagedLoad {
        names,
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
        fixture::{name_fixture, name_fixture_full},
        transactions::instantiate_test_contract,
    };

    use super::*;

    type TestDeps = OwnedDeps<MemoryStorage, MockApi, MockQuerier>;

    #[rstest::fixture]
    fn deps() -> TestDeps {
        instantiate_test_contract()
    }

    #[rstest::fixture]
    fn uniq_names() -> Vec<RegisteredName> {
        vec![
            name_fixture_full(1, "one", "address.one@a", "owner_one"),
            name_fixture_full(2, "two", "address.two@b", "owner_two"),
            name_fixture_full(3, "three", "address.three@c", "owner_three"),
        ]
    }

    #[rstest::fixture]
    fn overlapping_addresses() -> Vec<RegisteredName> {
        vec![
            name_fixture_full(1, "one", "address.one@a", "owner_one"),
            name_fixture_full(2, "two", "address.two@b", "owner_two"),
            name_fixture_full(3, "three", "address.two@b", "owner_three"),
        ]
    }

    #[rstest::fixture]
    fn overlapping_owners() -> Vec<RegisteredName> {
        vec![
            name_fixture_full(1, "one", "address.one@a", "owner_one"),
            name_fixture_full(2, "two", "address.two@b", "owner_two"),
            name_fixture_full(3, "three", "address.three@c", "owner_two"),
        ]
    }

    fn assert_not_registered(store: &dyn Storage, names: Vec<RegisteredName>) {
        let loaded = load_all_paged(store, None, None).unwrap();
        for name in &names {
            assert!(!has_name_id(store, name.id));
            assert!(!has_name(store, name.entry()));
            assert!(!loaded.names.iter().any(|l_name| l_name.id == name.id));
            assert!(!loaded.names.iter().any(|l_name| l_name == name));
        }
    }

    fn assert_registered(store: &dyn Storage, names: Vec<RegisteredName>) {
        let loaded = load_all_paged(store, None, None).unwrap();
        for name in &names {
            assert!(has_name_id(store, name.id));
            assert!(has_name(store, name.entry()));
            assert!(loaded.names.iter().filter(|n| n == &name).count() == 1);
        }
    }

    fn assert_only_these_registered(store: &dyn Storage, names: Vec<RegisteredName>) {
        for name in &names {
            assert!(has_name_id(store, name.id));
            assert!(has_name(store, name.entry()));
        }
        let last_id = names.last().unwrap().id;
        assert_eq!(
            load_all_paged(store, None, None).unwrap(),
            PagedLoad {
                names,
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(last_id),
            }
        )
    }

    #[rstest]
    fn single_basic_save_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
    }

    #[rstest]
    fn save_same_name_twice_fails(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert!(matches!(
            save(deps.as_mut().storage, &name_fixture(2)).unwrap_err(),
            NameServiceError::Std(StdError::GenericErr { .. })
        ));
    }

    #[rstest]
    fn remove_id_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert!(has_name_id(&deps.storage, 1));
        assert!(has_name(&deps.storage, name_fixture(1).entry()));
        remove_id(deps.as_mut().storage, 1).unwrap();
        assert!(!has_name_id(&deps.storage, 1));
        assert!(!has_name(&deps.storage, name_fixture(1).entry()));
    }

    #[rstest]
    fn remove_name_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert!(has_name_id(&deps.storage, 1));
        assert!(has_name(&deps.storage, name_fixture(1).entry()));
        remove_name(deps.as_mut().storage, name_fixture(1).name.name).unwrap();
        assert!(!has_name_id(&deps.storage, 1));
        assert!(!has_name(&deps.storage, name_fixture(1).entry()));
    }

    #[rstest]
    fn has_name_works(mut deps: TestDeps) {
        assert!(!has_name(&deps.storage, name_fixture(1).entry()));
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert!(has_name(&deps.storage, name_fixture(1).entry()));
    }

    #[rstest]
    fn has_name_id_works(mut deps: TestDeps) {
        assert!(!has_name_id(&deps.storage, 1));
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert!(has_name_id(&deps.storage, 1));
    }

    #[rstest]
    fn has_name_id_with_incorrect_id_fails(mut deps: TestDeps) {
        assert!(!has_name_id(&deps.storage, 2));
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert!(!has_name_id(&deps.storage, 2));
    }

    #[rstest]
    fn load_id_works(mut deps: TestDeps) {
        assert_eq!(
            load_id(deps.as_ref().storage, 1).unwrap_err(),
            NameServiceError::NotFound { name_id: 1 }
        );
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert_eq!(load_id(deps.as_ref().storage, 1).unwrap(), name_fixture(1),);
    }

    #[rstest]
    fn load_name_works(mut deps: TestDeps) {
        assert_eq!(
            load_name(deps.as_ref().storage, name_fixture(1).entry()).unwrap_err(),
            NameServiceError::NameNotFound {
                name: name_fixture(1).name.name,
            }
        );
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert_eq!(
            load_name(deps.as_ref().storage, name_fixture(1).entry()).unwrap(),
            name_fixture(1),
        );
    }

    #[rstest]
    fn load_address_works(mut deps: TestDeps) {
        assert_eq!(
            load_address(deps.as_ref().storage, &name_fixture(1).name.address).unwrap(),
            vec![],
        );
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert_eq!(
            load_address(deps.as_ref().storage, &name_fixture(1).name.address).unwrap(),
            vec![name_fixture(1)],
        );
    }

    #[rstest]
    fn load_owner_works(mut deps: TestDeps) {
        assert_eq!(
            load_owner(deps.as_ref().storage, name_fixture(1).owner).unwrap(),
            vec![],
        );
        save(deps.as_mut().storage, &name_fixture(1)).unwrap();
        assert_eq!(
            load_owner(deps.as_ref().storage, name_fixture(1).owner).unwrap(),
            vec![name_fixture(1)],
        );
    }

    #[rstest]
    fn load_all_paged_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        assert_eq!(
            load_all_paged(&deps.storage, None, None).unwrap(),
            PagedLoad {
                names: vec![],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: None
            }
        );
        save(deps.as_mut().storage, &uniq_names[0]).unwrap();
        assert_eq!(
            load_all_paged(&deps.storage, None, None).unwrap(),
            PagedLoad {
                names: vec![uniq_names[0].clone()],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(1),
            }
        );
    }

    #[rstest]
    fn save_set_of_unique_names_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        assert_not_registered(&deps.storage, uniq_names.clone());
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        assert_registered(&deps.storage, uniq_names.clone());
        assert_only_these_registered(&deps.storage, uniq_names);
    }

    #[rstest]
    fn load_name_for_unique_set_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        for name in uniq_names {
            assert_eq!(
                load_name(deps.as_ref().storage, name.entry()).unwrap(),
                name.clone(),
            );
        }
    }

    #[rstest]
    fn save_and_remove_name_id_from_set_of_unique_set_works(
        mut deps: TestDeps,
        uniq_names: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        remove_id(deps.as_mut().storage, 2).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(3, "three", "address.three@c", "owner_three"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
    }

    #[rstest]
    fn save_and_remove_name_from_set_of_unique_set_works(
        mut deps: TestDeps,
        uniq_names: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        remove_name(deps.as_mut().storage, uniq_names[1].entry().clone()).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(3, "three", "address.three@c", "owner_three"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
    }

    #[rstest]
    fn save_set_of_overlapping_addressed_works(
        mut deps: TestDeps,
        overlapping_addresses: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_addresses).unwrap();
        assert_registered(&deps.storage, overlapping_addresses.clone());
        assert_only_these_registered(&deps.storage, overlapping_addresses);
    }

    #[rstest]
    fn load_address_with_overlapping_addresses_works(
        mut deps: TestDeps,
        overlapping_addresses: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_addresses).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(2, "two", "address.two@b", "owner_two"),
                    name_fixture_full(3, "three", "address.two@b", "owner_three"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
        assert_eq!(
            load_address(
                deps.as_ref().storage,
                &Address::new("address.two@b").unwrap()
            )
            .unwrap(),
            vec![
                name_fixture_full(2, "two", "address.two@b", "owner_two"),
                name_fixture_full(3, "three", "address.two@b", "owner_three"),
            ]
        );
    }

    #[rstest]
    fn save_and_remove_name_id_from_set_of_overlapping_addresses_works(
        mut deps: TestDeps,
        overlapping_addresses: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_addresses).unwrap();
        remove_id(deps.as_mut().storage, 2).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(3, "three", "address.two@b", "owner_three"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
    }

    #[rstest]
    fn save_and_remove_name_from_set_of_overlapping_addresses_works(
        mut deps: TestDeps,
        overlapping_addresses: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_addresses).unwrap();
        remove_name(
            deps.as_mut().storage,
            overlapping_addresses[1].name.name.clone(),
        )
        .unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(3, "three", "address.two@b", "owner_three"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
    }

    #[rstest]
    fn save_set_of_overlapping_owners_works(
        mut deps: TestDeps,
        overlapping_owners: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_registered(&deps.storage, overlapping_owners.clone());
        assert_only_these_registered(&deps.storage, overlapping_owners);
    }

    #[rstest]
    fn load_owner_with_overlapping_owners_works(
        mut deps: TestDeps,
        overlapping_owners: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(2, "two", "address.two@b", "owner_two"),
                    name_fixture_full(3, "three", "address.three@c", "owner_two"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
        assert_eq!(
            load_owner(deps.as_ref().storage, Addr::unchecked("owner_two")).unwrap(),
            vec![
                name_fixture_full(2, "two", "address.two@b", "owner_two"),
                name_fixture_full(3, "three", "address.three@c", "owner_two"),
            ]
        );
    }

    #[rstest]
    fn save_and_remove_name_id_from_set_of_overlapping_owners_works(
        mut deps: TestDeps,
        overlapping_owners: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_registered(&deps.storage, overlapping_owners.clone());
        assert_only_these_registered(&deps.storage, overlapping_owners);
        remove_id(deps.as_mut().storage, 2).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(3, "three", "address.three@c", "owner_two"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
    }

    #[rstest]
    fn save_and_remove_name_from_set_of_overlapping_owners_works(
        mut deps: TestDeps,
        overlapping_owners: Vec<RegisteredName>,
    ) {
        save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_registered(&deps.storage, overlapping_owners.clone());
        assert_only_these_registered(&deps.storage, overlapping_owners.clone());
        remove_name(deps.as_mut().storage, overlapping_owners[1].entry().clone()).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(3, "three", "address.three@c", "owner_two"),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
    }

    #[rstest]
    fn load_all_paged_with_limit_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), None).unwrap(),
            PagedLoad {
                names: vec![
                    name_fixture_full(1, "one", "address.one@a", "owner_one"),
                    name_fixture_full(2, "two", "address.two@b", "owner_two"),
                ],
                limit: 2,
                start_next_after: Some(2),
            }
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(1), Some(2)).unwrap(),
            PagedLoad {
                names: vec![name_fixture_full(
                    3,
                    "three",
                    "address.three@c",
                    "owner_three"
                )],
                limit: 1,
                start_next_after: Some(3),
            }
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), Some(2)).unwrap(),
            PagedLoad {
                names: vec![name_fixture_full(
                    3,
                    "three",
                    "address.three@c",
                    "owner_three"
                )],
                limit: 2,
                start_next_after: Some(3),
            }
        );
    }

    #[test]
    #[ignore]
    fn max_page_limit_is_applied() {
        todo!();
    }
}
