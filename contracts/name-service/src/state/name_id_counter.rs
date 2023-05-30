use cosmwasm_std::Storage;
use cw_storage_plus::Item;
use nym_name_service_common::NameId;

use crate::{constants::NAME_ID_COUNTER_KEY, error::Result};

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

    use nym_name_service_common::NameEntry;

    use crate::test_helpers::{
        assert::assert_names,
        fixture::name_fixture_name,
        helpers::{delete_name_id, instantiate_test_contract, register_name},
    };

    #[test]
    fn get_next_name_id() {
        let mut deps = instantiate_test_contract();

        assert_eq!(register_name(deps.as_mut(), &name_fixture_name("foo")), 1);
        assert_names(
            deps.as_ref(),
            &[NameEntry::new(1, name_fixture_name("foo"))],
        );

        assert_eq!(register_name(deps.as_mut(), &name_fixture_name("bar")), 2);
        assert_eq!(register_name(deps.as_mut(), &name_fixture_name("baz")), 3);
        assert_names(
            deps.as_ref(),
            &[
                NameEntry::new(1, name_fixture_name("foo")),
                NameEntry::new(2, name_fixture_name("bar")),
                NameEntry::new(3, name_fixture_name("baz")),
            ],
        );
    }

    #[test]
    fn deleted_name_id_is_not_reused() {
        let mut deps = instantiate_test_contract();

        // Register two names
        assert_eq!(register_name(deps.as_mut(), &name_fixture_name("one")), 1);
        assert_eq!(register_name(deps.as_mut(), &name_fixture_name("two")), 2);
        assert_names(
            deps.as_ref(),
            &[
                NameEntry::new(1, name_fixture_name("one")),
                NameEntry::new(2, name_fixture_name("two")),
            ],
        );

        // Delete the last entry
        delete_name_id(deps.as_mut(), 2, "steve");
        assert_names(
            deps.as_ref(),
            &[NameEntry::new(1, name_fixture_name("one"))],
        );

        // Create a third entry. The index should not reuse the previous entry that we just
        // deleted.
        assert_eq!(register_name(deps.as_mut(), &name_fixture_name("two")), 3);
        assert_names(
            deps.as_ref(),
            &[
                NameEntry::new(1, name_fixture_name("one")),
                NameEntry::new(3, name_fixture_name("two")),
            ],
        );
    }
}
