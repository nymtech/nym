use cosmwasm_std::Storage;
use cw_storage_plus::Item;
use nym_name_service_common::NameId;

use crate::{constants::NAME_ID_COUNTER_KEY, Result};

const NAME_ID_COUNTER: Item<NameId> = Item::new(NAME_ID_COUNTER_KEY);

/// Generate the next name id, store it and return it
pub(crate) fn next_name_id_counter(store: &mut dyn Storage) -> Result<NameId> {
    // The first id is 1.
    let id = NAME_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    NAME_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::Addr;
    use nym_name_service_common::RegisteredName;

    use crate::test_helpers::{
        assert::assert_names,
        helpers::{nyms, test_rng},
        transactions::{delete_name_id, instantiate_test_contract, register_name},
    };

    #[test]
    fn get_next_name_id() {
        let mut deps = instantiate_test_contract();
        let mut rng = test_rng();

        let (id1, name1) = register_name(deps.as_mut(), &mut rng, "foo", "steve");
        let (id2, name2) = register_name(deps.as_mut(), &mut rng, "bar", "steve");
        let (id3, name3) = register_name(deps.as_mut(), &mut rng, "baz", "steve");
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_eq!(name1.name.as_str(), "foo");
        assert_eq!(name2.name.as_str(), "bar");
        assert_eq!(name3.name.as_str(), "baz");
        assert_names(
            deps.as_ref(),
            &[
                RegisteredName {
                    id: 1,
                    name: name1,
                    owner: Addr::unchecked("steve"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
                RegisteredName {
                    id: 2,
                    name: name2,
                    owner: Addr::unchecked("steve"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
                RegisteredName {
                    id: 3,
                    name: name3,
                    owner: Addr::unchecked("steve"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            ],
        );
    }

    #[test]
    fn deleted_name_id_is_not_reused() {
        let mut deps = instantiate_test_contract();
        let mut rng = test_rng();

        // Register two names
        let (_, name1) = register_name(deps.as_mut(), &mut rng, "one", "steve");
        register_name(deps.as_mut(), &mut rng, "two", "steve");

        // Delete the last entry
        delete_name_id(deps.as_mut(), 2, "steve");
        assert_names(
            deps.as_ref(),
            &[RegisteredName {
                id: 1,
                name: name1,
                owner: Addr::unchecked("steve"),
                block_height: 12345,
                deposit: nyms(100),
            }],
        );

        // Create a third entry. The index should not reuse the previous entry that we just
        // deleted.
        let (id3, _) = register_name(deps.as_mut(), &mut rng, "three", "steve");
        assert_eq!(id3, 3);
    }
}
