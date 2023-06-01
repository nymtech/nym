use cosmwasm_std::Addr;
use nym_service_provider_directory_common::{response::PagedServicesListResponse, NymAddress};

use crate::{
    constants::SERVICE_DEFAULT_RETRIEVAL_LIMIT,
    test_helpers::{fixture::new_service, helpers::nyms},
    SpContractError,
};

use super::test_setup::TestSetup;

#[test]
fn delete_service() {
    let mut setup = TestSetup::new();
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress"),
        &Addr::unchecked("announcer"),
        &nyms(100),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance("announcer"), nyms(150));
    assert!(!setup.query_all().services.is_empty());
    setup.delete(1, &Addr::unchecked("announcer"));

    // Deleting the service returns the deposit to the announcer
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance("announcer"), nyms(250));
    assert!(setup.query_all().services.is_empty());
}

#[test]
fn only_announcer_can_delete_service() {
    let mut setup = TestSetup::new();
    assert_eq!(setup.contract_balance(), nyms(0));
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress"),
        &Addr::unchecked("announcer"),
        &nyms(100),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert!(!setup.query_all().services.is_empty());

    let delete_resp: SpContractError = setup
        .try_delete(1, &Addr::unchecked("not_announcer"))
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(
        delete_resp,
        SpContractError::Unauthorized {
            sender: Addr::unchecked("not_announcer")
        }
    );
}

#[test]
fn cant_delete_service_that_does_not_exist() {
    let mut setup = TestSetup::new();
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress"),
        &Addr::unchecked("announcer"),
        &nyms(100),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert!(!setup.query_all().services.is_empty());

    let delete_resp: SpContractError = setup
        .try_delete(0, &Addr::unchecked("announcer"))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(delete_resp, SpContractError::NotFound { service_id: 0 });

    let delete_resp: SpContractError = setup
        .try_delete(2, &Addr::unchecked("announcer"))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(delete_resp, SpContractError::NotFound { service_id: 2 });

    assert!(!setup.query_all().services.is_empty());
    setup.delete(1, &Addr::unchecked("announcer"));
    assert_eq!(setup.contract_balance(), nyms(0));
    assert!(setup.query_all().services.is_empty());
}

#[test]
fn announce_multiple_services_and_deleting_by_name() {
    let mut setup = TestSetup::new();
    let announcer1 = Addr::unchecked("wealthy_announcer_1");
    let announcer2 = Addr::unchecked("wealthy_announcer_2");
    let nym_address1 = NymAddress::new("nymAddress1");
    let nym_address2 = NymAddress::new("nymAddress2");
    let deposit = nyms(100);

    // We announce the same address three times, but with different annoucers
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&announcer1), nyms(1000));
    let s1 = setup.sign_and_announce_net_req(&nym_address1, &announcer1, &deposit);
    let s2 = setup.sign_and_announce_net_req(&nym_address1, &announcer1, &deposit);
    let s3 = setup.sign_and_announce_net_req(&nym_address2, &announcer1, &deposit);
    let s4 = setup.sign_and_announce_net_req(&nym_address1, &announcer2, &deposit);
    let s5 = setup.sign_and_announce_net_req(&nym_address2, &announcer2, &deposit);

    assert_eq!(setup.contract_balance(), nyms(500));
    assert_eq!(setup.balance(&announcer1), nyms(700));
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![
                new_service(1, &nym_address1, &announcer1, s1.identity_key()),
                new_service(2, &nym_address1, &announcer1, s2.identity_key()),
                new_service(3, &nym_address2, &announcer1, s3.identity_key()),
                new_service(4, &nym_address1, &announcer2, s4.identity_key()),
                new_service(5, &nym_address2, &announcer2, s5.identity_key()),
            ],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );

    // Even though multiple of them point to the same nym address, we only delete the ones we actually
    // own.
    setup.delete_nym_address(&nym_address1, &announcer1);

    assert_eq!(setup.contract_balance(), nyms(300));
    assert_eq!(setup.balance(&announcer1), nyms(900));
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![
                new_service(3, &nym_address2, &announcer1, s3.identity_key()),
                new_service(4, &nym_address1, &announcer2, s4.identity_key()),
                new_service(5, &nym_address2, &announcer2, s5.identity_key()),
            ],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );
}
