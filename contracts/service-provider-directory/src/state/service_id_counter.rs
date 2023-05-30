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

    use cosmwasm_std::Addr;
    use nym_service_provider_directory_common::Service;

    use crate::test_helpers::{
        assert::assert_services,
        helpers::{nyms, test_rng},
        transactions::{announce_service, delete_service, instantiate_test_contract},
    };

    #[test]
    fn get_next_service_id() {
        let mut deps = instantiate_test_contract();
        let mut rng = test_rng();

        let (id1, service1) = announce_service(deps.as_mut(), &mut rng, "addr1", "timmy");
        let (id2, service2) = announce_service(deps.as_mut(), &mut rng, "addr2", "timmy");
        let (id3, service3) = announce_service(deps.as_mut(), &mut rng, "addr3", "timmy");
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
        assert_services(
            deps.as_ref(),
            &[
                Service {
                    service_id: 1,
                    service: service1,
                    announcer: Addr::unchecked("timmy"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
                Service {
                    service_id: 2,
                    service: service2,
                    announcer: Addr::unchecked("timmy"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
                Service {
                    service_id: 3,
                    service: service3,
                    announcer: Addr::unchecked("timmy"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            ],
        );
    }

    #[test]
    fn deleted_service_id_is_not_reused() {
        let mut deps = instantiate_test_contract();
        let mut rng = test_rng();

        // Announce
        let (_, service1) = announce_service(deps.as_mut(), &mut rng, "addr1", "timmy");
        let _ = announce_service(deps.as_mut(), &mut rng, "addr2", "timmy");

        //// Delete the last entry
        delete_service(deps.as_mut(), 2, "timmy");
        assert_services(
            deps.as_ref(),
            &[Service {
                service_id: 1,
                service: service1,
                announcer: Addr::unchecked("timmy"),
                block_height: 12345,
                deposit: nyms(100),
            }],
        );

        // Create a third entry. The index should not reuse the previous entry that we just
        // deleted.
        let (id3, _) = announce_service(deps.as_mut(), &mut rng, "addr3", "timmy");
        assert_eq!(id3, 3);
    }
}
