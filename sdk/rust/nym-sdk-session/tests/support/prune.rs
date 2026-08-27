use nym_bandwidth_controller::{BandwidthController, TicketType};
use nym_credential_storage::{initialise_ephemeral_storage, storage::Storage};
use time::Date;

use crate::support::TestEcash;

/// Calling prune on the bandwidth controller frees up expected data in ticketbook storage,
/// as well as in pending storage.
#[tokio::test]
async fn prune_empty_storage() {
    let ecash = TestEcash::new();
    let storage = initialise_ephemeral_storage();
    let controller = BandwidthController::new(storage.clone());

    // pruning on empty storage doesn't error
    controller.prune_expired().await;

    // pruning old ticketbooks leaves the storage empty
    let ticketbook = ecash.ticketbook_with_expiration(
        TicketType::V1WireguardEntry,
        42,
        Date::from_calendar_date(2000, 1.try_into().unwrap(), 1).unwrap(),
    );
    storage.insert_issued_ticketbook(&ticketbook).await.unwrap();
    controller.prune_expired().await;
    assert_eq!(storage.get_ticketbooks_info().await.unwrap().len(), 0);

    // pruning non-expired ticketbooks doesn't touch them
    let ticketbook = ecash.ticketbook_with_expiration(
        TicketType::V1WireguardEntry,
        42,
        Date::from_calendar_date(2100, 1.try_into().unwrap(), 1).unwrap(),
    );
    storage.insert_issued_ticketbook(&ticketbook).await.unwrap();
    controller.prune_expired().await;
    assert_ne!(storage.get_ticketbooks_info().await.unwrap().len(), 0);
}
