use cosmwasm_std::Storage;
use cw_storage_plus::Item;
use nym_service_provider_directory_common::ServiceId;

use crate::{constants::SERVICE_ID_COUNTER_KEY, error::Result};

const SERVICE_ID_COUNTER: Item<ServiceId> = Item::new(SERVICE_ID_COUNTER_KEY);

/// Generate the next service provider id, store it and return it
pub(crate) fn next_service_id_counter(store: &mut dyn Storage) -> Result<ServiceId> {
    // The first id is 1.
    let id = SERVICE_ID_COUNTER.may_load(store)?.unwrap_or_default() + 1;
    SERVICE_ID_COUNTER.save(store, &id)?;
    Ok(id)
}

#[cfg(test)]
mod tests {

    use nym_service_provider_directory_common::msg::ServiceInfo;

    use crate::test_helpers::{
        assert::assert_services,
        fixture::service_fixture,
        helpers::{announce_service, delete_service, instantiate_test_contract},
    };

    #[test]
    fn get_next_service_id() {
        let mut deps = instantiate_test_contract();

        assert_eq!(announce_service(deps.as_mut(), service_fixture()), 1);
        assert_services(deps.as_ref(), &[ServiceInfo::new(1, service_fixture())]);

        assert_eq!(announce_service(deps.as_mut(), service_fixture()), 2);
        assert_eq!(announce_service(deps.as_mut(), service_fixture()), 3);
        assert_services(
            deps.as_ref(),
            &[
                ServiceInfo::new(1, service_fixture()),
                ServiceInfo::new(2, service_fixture()),
                ServiceInfo::new(3, service_fixture()),
            ],
        );
    }

    #[test]
    fn deleted_service_id_is_not_reused() {
        let mut deps = instantiate_test_contract();

        // Announce
        assert_eq!(announce_service(deps.as_mut(), service_fixture()), 1);
        assert_eq!(announce_service(deps.as_mut(), service_fixture()), 2);
        assert_services(
            deps.as_ref(),
            &[
                ServiceInfo::new(1, service_fixture()),
                ServiceInfo::new(2, service_fixture()),
            ],
        );

        // Delete the last entry
        delete_service(deps.as_mut(), 2, "steve");
        assert_services(deps.as_ref(), &[ServiceInfo::new(1, service_fixture())]);

        // Create a third entry. The index should not reuse the previous entry that we just
        // deleted.
        assert_eq!(announce_service(deps.as_mut(), service_fixture()), 3);
        assert_services(
            deps.as_ref(),
            &[
                ServiceInfo::new(1, service_fixture()),
                ServiceInfo::new(3, service_fixture()),
            ],
        );
    }
}
