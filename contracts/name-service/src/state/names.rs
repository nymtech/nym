use cosmwasm_std::{Addr, Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Index, IndexList, IndexedMap, MultiIndex, UniqueIndex};
use nym_name_service_common::{Address, NameId, NymName, RegisteredName};

use crate::{
    constants::{
        MAX_NUMBER_OF_NAMES_FOR_ADDRESS, MAX_NUMBER_OF_NAMES_PER_OWNER,
        NAMES_ADDRESS_IDX_NAMESPACE, NAMES_NAME_IDX_NAMESPACE, NAMES_OWNER_IDX_NAMESPACE,
        NAMES_PK_NAMESPACE, NAME_DEFAULT_RETRIEVAL_LIMIT, NAME_MAX_RETRIEVAL_LIMIT,
    },
    error::{NameServiceError, Result},
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
        name: UniqueIndex::new(|d| d.name.to_string(), NAMES_NAME_IDX_NAMESPACE),
        address: MultiIndex::new(
            |d| d.address.to_string(),
            NAMES_PK_NAMESPACE,
            NAMES_ADDRESS_IDX_NAMESPACE,
        ),
        owner: MultiIndex::new(
            |d| d.owner.clone(),
            NAMES_PK_NAMESPACE,
            NAMES_OWNER_IDX_NAMESPACE,
        ),
    };
    IndexedMap::new(NAMES_PK_NAMESPACE, indexes)
}

pub fn save(store: &mut dyn Storage, new_name: &RegisteredName) -> Result<NameId> {
    let name_id = super::next_name_id_counter(store)?;
    names().save(store, name_id, new_name)?;
    Ok(name_id)
}

#[cfg(test)]
pub fn save_all(state: &mut dyn Storage, names: &[RegisteredName]) -> Result<Vec<NameId>> {
    let mut ids = vec![];
    for name in names {
        ids.push(save(state, name)?);
    }
    Ok(ids)
}

pub fn has_name_id(store: &dyn Storage, name_id: NameId) -> bool {
    names().has(store, name_id)
}

pub fn has_name(store: &dyn Storage, name: &NymName) -> bool {
    load_name(store, name).is_ok()
}

// Get the (key, name) entry for a given name
pub fn load_name_entry(store: &dyn Storage, name: &NymName) -> Result<(NameId, RegisteredName)> {
    names()
        .idx
        .name
        .range(store, None, None, Order::Ascending)
        .find(|entry| {
            if let Ok(entry) = entry {
                &entry.1.name == name
            } else {
                false
            }
        })
        .ok_or(NameServiceError::NameNotFound { name: name.clone() })?
        .map_err(NameServiceError::from)
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

pub fn load_address(
    store: &dyn Storage,
    address: &Address,
) -> Result<Vec<(NameId, RegisteredName)>> {
    names()
        .idx
        .address
        .prefix(address.to_string())
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_NAMES_FOR_ADDRESS as usize)
        .collect::<StdResult<Vec<_>>>()
        .map_err(NameServiceError::from)
}

pub fn load_owner(store: &dyn Storage, owner: Addr) -> Result<Vec<(NameId, RegisteredName)>> {
    names()
        .idx
        .owner
        .prefix(owner)
        .range(store, None, None, Order::Ascending)
        .take(MAX_NUMBER_OF_NAMES_PER_OWNER as usize)
        .collect::<StdResult<Vec<_>>>()
        .map_err(NameServiceError::from)
}

pub fn remove_id(store: &mut dyn Storage, name_id: NameId) -> Result<()> {
    Ok(names().remove(store, name_id)?)
}

#[cfg(test)]
pub fn remove_name(store: &mut dyn Storage, name: NymName) -> Result<NameId> {
    let name_info = load_name_entry(store, &name)?;
    remove_id(store, name_info.0)?;
    Ok(name_info.0)
}

#[derive(Debug, PartialEq)]
pub struct PagedLoad {
    pub names: Vec<(NameId, RegisteredName)>,
    pub limit: usize,
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
        .collect::<StdResult<Vec<_>>>()?;

    let start_next_after = names.last().map(|name| name.0);

    Ok(PagedLoad {
        names,
        limit,
        start_next_after,
    })
}

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use cosmwasm_std::{
        testing::{MockApi, MockQuerier},
        MemoryStorage, OwnedDeps,
    };
    use rstest::rstest;

    use crate::{
        error::NameServiceError,
        test_helpers::{
            fixture::{name_fixture, name_fixture_full},
            transactions::instantiate_test_contract,
        },
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
            name_fixture_full("one", "address_one", "owner_one"),
            name_fixture_full("two", "address_two", "owner_two"),
            name_fixture_full("three", "address_three", "owner_three"),
        ]
    }

    #[rstest::fixture]
    fn overlapping_addresses() -> Vec<RegisteredName> {
        vec![
            name_fixture_full("one", "address_one", "owner_one"),
            name_fixture_full("two", "address_two", "owner_two"),
            name_fixture_full("three", "address_two", "owner_three"),
        ]
    }

    #[rstest::fixture]
    fn overlapping_owners() -> Vec<RegisteredName> {
        vec![
            name_fixture_full("one", "address_one", "owner_one"),
            name_fixture_full("two", "address_two", "owner_two"),
            name_fixture_full("three", "address_three", "owner_two"),
        ]
    }

    fn assert_not_registered(store: &dyn Storage, names: Vec<RegisteredName>, ids: Vec<NameId>) {
        let names: Vec<(NameId, RegisteredName)> = zip(ids, names).collect();
        let loaded = load_all_paged(store, None, None).unwrap();
        for (id, name) in &names {
            assert!(!has_name_id(store, *id));
            assert!(!has_name(store, &name.name));
            assert!(!loaded.names.iter().any(|(i, _)| i == id));
            assert!(!loaded.names.iter().any(|(_, n)| n == name));
        }
    }

    fn assert_registered(store: &dyn Storage, names: Vec<RegisteredName>, ids: Vec<NameId>) {
        assert!(names.len() == ids.len());
        let names: Vec<(NameId, RegisteredName)> = zip(ids, names).collect();
        let loaded = load_all_paged(store, None, None).unwrap();
        for (id, name) in &names {
            assert!(has_name_id(store, *id));
            assert!(has_name(store, &name.name));
            assert!(loaded.names.iter().filter(|(i, _)| i == id).count() == 1);
            assert!(loaded.names.iter().filter(|(_, n)| n == name).count() == 1);
        }
    }

    fn assert_only_these_registered(
        store: &dyn Storage,
        names: Vec<RegisteredName>,
        ids: Vec<NameId>,
    ) {
        let last_id = *ids.last().unwrap();
        let names: Vec<(NameId, RegisteredName)> = zip(ids, names).collect();
        for (id, name) in &names {
            assert!(has_name_id(store, *id));
            assert!(has_name(store, &name.name));
        }
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
        save(deps.as_mut().storage, &name_fixture()).unwrap();
    }

    #[rstest]
    fn save_same_name_twice_fails(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert!(matches!(
            save(deps.as_mut().storage, &name_fixture()).unwrap_err(),
            NameServiceError::Std(StdError::GenericErr { .. })
        ));
    }

    #[rstest]
    fn has_name_works(mut deps: TestDeps) {
        assert!(!has_name(&deps.storage, &name_fixture().name));
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert!(has_name(&deps.storage, &name_fixture().name));
    }

    #[rstest]
    fn has_name_id_works(mut deps: TestDeps) {
        assert!(!has_name_id(&deps.storage, 1));
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert!(has_name_id(&deps.storage, 1));
    }

    #[rstest]
    fn has_name_id_with_incorrect_id_fails(mut deps: TestDeps) {
        assert!(!has_name_id(&deps.storage, 2));
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert!(!has_name_id(&deps.storage, 2));
    }

    #[rstest]
    fn load_name_entry_works(mut deps: TestDeps) {
        assert_eq!(
            load_name_entry(deps.as_ref().storage, &name_fixture().name).unwrap_err(),
            NameServiceError::NameNotFound {
                name: name_fixture().name
            }
        );
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert_eq!(
            load_name_entry(deps.as_ref().storage, &name_fixture().name).unwrap(),
            (1, name_fixture())
        );
    }

    #[rstest]
    fn load_id_works(mut deps: TestDeps) {
        assert_eq!(
            load_id(deps.as_ref().storage, 1).unwrap_err(),
            NameServiceError::NotFound { name_id: 1 }
        );
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert_eq!(load_id(deps.as_ref().storage, 1).unwrap(), name_fixture(),);
    }

    #[rstest]
    fn load_name_works(mut deps: TestDeps) {
        assert_eq!(
            load_name(deps.as_ref().storage, &name_fixture().name).unwrap_err(),
            NameServiceError::NameNotFound {
                name: name_fixture().name
            }
        );
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert_eq!(
            load_name(deps.as_ref().storage, &name_fixture().name).unwrap(),
            name_fixture(),
        );
    }

    #[rstest]
    fn load_address_works(mut deps: TestDeps) {
        assert_eq!(
            load_address(deps.as_ref().storage, &name_fixture().address).unwrap(),
            vec![],
        );
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert_eq!(
            load_address(deps.as_ref().storage, &name_fixture().address).unwrap(),
            vec![(1, name_fixture())],
        );
    }

    #[rstest]
    fn load_owner_works(mut deps: TestDeps) {
        assert_eq!(
            load_owner(deps.as_ref().storage, name_fixture().owner).unwrap(),
            vec![],
        );
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert_eq!(
            load_owner(deps.as_ref().storage, name_fixture().owner).unwrap(),
            vec![(1, name_fixture())],
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
                names: vec![(1, uniq_names[0].clone())],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(1),
            }
        );
    }

    #[rstest]
    fn remove_id_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert!(has_name_id(&deps.storage, 1));
        assert!(has_name(&deps.storage, &name_fixture().name));
        remove_id(deps.as_mut().storage, 1).unwrap();
        assert!(!has_name_id(&deps.storage, 1));
        assert!(!has_name(&deps.storage, &name_fixture().name));
    }

    #[rstest]
    fn remove_name_works(mut deps: TestDeps) {
        save(deps.as_mut().storage, &name_fixture()).unwrap();
        assert!(has_name_id(&deps.storage, 1));
        assert!(has_name(&deps.storage, &name_fixture().name));
        remove_name(deps.as_mut().storage, name_fixture().name).unwrap();
        assert!(!has_name_id(&deps.storage, 1));
        assert!(!has_name(&deps.storage, &name_fixture().name));
    }

    #[rstest]
    fn save_set_of_unique_names_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        let num = uniq_names.len() as NameId;
        let ids = (1..=num).collect::<Vec<NameId>>();
        assert_not_registered(&deps.storage, uniq_names.clone(), ids.clone());
        let saved_ids = save_all(deps.as_mut().storage, &uniq_names).unwrap();
        assert_eq!(saved_ids, ids);
        assert_registered(&deps.storage, uniq_names.clone(), ids.clone());
        assert_only_these_registered(&deps.storage, uniq_names, ids);
    }

    #[rstest]
    fn save_set_of_unique_names_generates_ids(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        let num = uniq_names.len() as NameId;
        let ids = save_all(deps.as_mut().storage, &uniq_names).unwrap();
        assert_eq!(ids, (1..=num).collect::<Vec<NameId>>());
    }

    #[rstest]
    fn load_name_entry_for_unique_set_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        for (id, name) in uniq_names.iter().enumerate() {
            assert_eq!(
                load_name_entry(deps.as_ref().storage, &name.name).unwrap(),
                (id as NameId + 1, name.clone()),
            );
        }
    }

    #[rstest]
    fn load_name_for_unique_set_works(mut deps: TestDeps, uniq_names: Vec<RegisteredName>) {
        save_all(deps.as_mut().storage, &uniq_names).unwrap();
        for name in uniq_names {
            assert_eq!(
                load_name(deps.as_ref().storage, &name.name).unwrap(),
                name.clone(),
            );
        }
    }

    #[rstest]
    fn save_and_remove_name_id_from_set_of_unique_set_works(
        mut deps: TestDeps,
        uniq_names: Vec<RegisteredName>,
    ) {
        let ids = save_all(deps.as_mut().storage, &uniq_names).unwrap();
        remove_id(deps.as_mut().storage, ids[1]).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (
                        3,
                        name_fixture_full("three", "address_three", "owner_three")
                    ),
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
        remove_name(deps.as_mut().storage, uniq_names[1].name.clone()).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (
                        3,
                        name_fixture_full("three", "address_three", "owner_three")
                    ),
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
        let ids = save_all(deps.as_mut().storage, &overlapping_addresses).unwrap();
        assert_registered(&deps.storage, overlapping_addresses.clone(), ids.clone());
        assert_only_these_registered(&deps.storage, overlapping_addresses, ids);
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
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (2, name_fixture_full("two", "address_two", "owner_two")),
                    (3, name_fixture_full("three", "address_two", "owner_three")),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
        assert_eq!(
            load_address(deps.as_ref().storage, &Address::new("address_two")).unwrap(),
            vec![
                (2, name_fixture_full("two", "address_two", "owner_two")),
                (3, name_fixture_full("three", "address_two", "owner_three")),
            ]
        );
    }

    #[rstest]
    fn save_and_remove_name_id_from_set_of_overlapping_addresses_works(
        mut deps: TestDeps,
        overlapping_addresses: Vec<RegisteredName>,
    ) {
        let ids = save_all(deps.as_mut().storage, &overlapping_addresses).unwrap();
        remove_id(deps.as_mut().storage, ids[1]).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (3, name_fixture_full("three", "address_two", "owner_three")),
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
        remove_name(deps.as_mut().storage, overlapping_addresses[1].name.clone()).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (3, name_fixture_full("three", "address_two", "owner_three")),
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
        let ids = save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_registered(&deps.storage, overlapping_owners.clone(), ids.clone());
        assert_only_these_registered(&deps.storage, overlapping_owners, ids);
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
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (2, name_fixture_full("two", "address_two", "owner_two")),
                    (3, name_fixture_full("three", "address_three", "owner_two")),
                ],
                limit: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
                start_next_after: Some(3),
            }
        );
        assert_eq!(
            load_owner(deps.as_ref().storage, Addr::unchecked("owner_two")).unwrap(),
            vec![
                (2, name_fixture_full("two", "address_two", "owner_two")),
                (3, name_fixture_full("three", "address_three", "owner_two")),
            ]
        );
    }

    #[rstest]
    fn save_and_remove_name_id_from_set_of_overlapping_owners_works(
        mut deps: TestDeps,
        overlapping_owners: Vec<RegisteredName>,
    ) {
        let ids = save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_registered(&deps.storage, overlapping_owners.clone(), ids.clone());
        assert_only_these_registered(&deps.storage, overlapping_owners, ids);
        remove_id(deps.as_mut().storage, 2).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (3, name_fixture_full("three", "address_three", "owner_two")),
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
        let ids = save_all(deps.as_mut().storage, &overlapping_owners).unwrap();
        assert_registered(&deps.storage, overlapping_owners.clone(), ids.clone());
        assert_only_these_registered(&deps.storage, overlapping_owners.clone(), ids);
        remove_name(deps.as_mut().storage, overlapping_owners[1].name.clone()).unwrap();
        assert_eq!(
            load_all_paged(deps.as_ref().storage, None, None).unwrap(),
            PagedLoad {
                names: vec![
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (3, name_fixture_full("three", "address_three", "owner_two")),
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
                    (1, name_fixture_full("one", "address_one", "owner_one")),
                    (2, name_fixture_full("two", "address_two", "owner_two")),
                ],
                limit: 2,
                start_next_after: Some(2),
            }
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(1), Some(2)).unwrap(),
            PagedLoad {
                names: vec![(
                    3,
                    name_fixture_full("three", "address_three", "owner_three")
                ),],
                limit: 1,
                start_next_after: Some(3),
            }
        );
        assert_eq!(
            load_all_paged(&deps.storage, Some(2), Some(2)).unwrap(),
            PagedLoad {
                names: vec![(
                    3,
                    name_fixture_full("three", "address_three", "owner_three")
                ),],
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
