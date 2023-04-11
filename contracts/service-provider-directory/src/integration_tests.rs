//! Integration tests using cw-multi-test.

use cosmwasm_std::Addr;
use nym_service_provider_directory_common::{
    response::{ConfigResponse, PagedServicesListResponse},
    NymAddress, Service, ServiceInfo, ServiceType,
};

use crate::{
    constants::SERVICE_DEFAULT_RETRIEVAL_LIMIT,
    error::ContractError,
    test_helpers::{fixture::service_info, helpers::nyms, test_setup::TestSetup},
};

#[test]
fn instantiate_contract() {
    TestSetup::new();
}

#[test]
fn query_config() {
    assert_eq!(
        TestSetup::new().query_config(),
        ConfigResponse {
            deposit_required: nyms(100),
        }
    );
}

#[test]
fn announce_and_query_service() {
    let mut setup = TestSetup::new();
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: None,
        }
    );

    // Announce a first service
    let announcer = Addr::unchecked("announcer");
    let nym_address = NymAddress::new("nymAddress");
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&announcer), nyms(250));
    setup.announce_net_req(nym_address.clone(), announcer.clone());

    // Deposit is deposited to contract and deducted from announcers's balance
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance(&announcer), nyms(150));

    // We can query the full service list
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![ServiceInfo {
                service_id: 1,
                service: Service {
                    nym_address: nym_address.clone(),
                    service_type: ServiceType::NetworkRequester,
                    announcer: announcer.clone(),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            }],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(1),
        }
    );

    // ... and we can query by id
    assert_eq!(
        setup.query_id(1),
        ServiceInfo {
            service_id: 1,
            service: Service {
                nym_address: nym_address.clone(),
                service_type: ServiceType::NetworkRequester,
                announcer: announcer.clone(),
                block_height: 12345,
                deposit: nyms(100),
            },
        }
    );

    // Announce a second service
    let announcer2 = Addr::unchecked("announcer2");
    let nym_address2 = NymAddress::new("nymAddress2");
    setup.announce_net_req(nym_address2.clone(), announcer2.clone());

    assert_eq!(setup.contract_balance(), nyms(200));
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![
                service_info(1, nym_address, announcer),
                service_info(2, nym_address2, announcer2)
            ],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(2),
        }
    );
}

#[test]
fn delete_service() {
    let mut setup = TestSetup::new();
    setup.announce_net_req(NymAddress::new("nymAddress"), Addr::unchecked("announcer"));
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance("announcer"), nyms(150));
    assert!(!setup.query_all().services.is_empty());
    setup.delete(1, Addr::unchecked("announcer"));

    // Deleting the service returns the deposit to the announcer
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance("announcer"), nyms(250));
    assert!(setup.query_all().services.is_empty());
}

#[test]
fn only_announcer_can_delete_service() {
    let mut setup = TestSetup::new();
    assert_eq!(setup.contract_balance(), nyms(0));
    setup.announce_net_req(NymAddress::new("nymAddress"), Addr::unchecked("announcer"));
    assert_eq!(setup.contract_balance(), nyms(100));
    assert!(!setup.query_all().services.is_empty());

    let delete_resp: ContractError = setup
        .try_delete(1, Addr::unchecked("not_announcer"))
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(
        delete_resp,
        ContractError::Unauthorized {
            sender: Addr::unchecked("not_announcer")
        }
    );
}

#[test]
fn cant_delete_service_that_does_not_exist() {
    let mut setup = TestSetup::new();
    setup.announce_net_req(NymAddress::new("nymAddress"), Addr::unchecked("announcer"));
    assert_eq!(setup.contract_balance(), nyms(100));
    assert!(!setup.query_all().services.is_empty());

    let delete_resp: ContractError = setup
        .try_delete(0, Addr::unchecked("announcer"))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(delete_resp, ContractError::NotFound { service_id: 0 });

    let delete_resp: ContractError = setup
        .try_delete(2, Addr::unchecked("announcer"))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(delete_resp, ContractError::NotFound { service_id: 2 });

    assert!(!setup.query_all().services.is_empty());
    setup.delete(1, Addr::unchecked("announcer"));
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

    // We announce the same address three times, but with different annoucers
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&announcer1), nyms(1000));
    setup.announce_net_req(nym_address1.clone(), announcer1.clone());
    setup.announce_net_req(nym_address1.clone(), announcer1.clone());
    setup.announce_net_req(nym_address2.clone(), announcer1.clone());
    setup.announce_net_req(nym_address1.clone(), announcer2.clone());
    setup.announce_net_req(nym_address2.clone(), announcer2.clone());

    assert_eq!(setup.contract_balance(), nyms(500));
    assert_eq!(setup.balance(&announcer1), nyms(700));
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![
                service_info(1, nym_address1.clone(), announcer1.clone()),
                service_info(2, nym_address1.clone(), announcer1.clone()),
                service_info(3, nym_address2.clone(), announcer1.clone()),
                service_info(4, nym_address1.clone(), announcer2.clone()),
                service_info(5, nym_address2.clone(), announcer2.clone()),
            ],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );

    // Even though multiple of them point to the same nym address, we only delete the ones we actually
    // own.
    setup.delete_nym_address(nym_address1.clone(), announcer1.clone());

    assert_eq!(setup.contract_balance(), nyms(300));
    assert_eq!(setup.balance(&announcer1), nyms(900));
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![
                service_info(3, nym_address2.clone(), announcer1),
                service_info(4, nym_address1, announcer2.clone()),
                service_info(5, nym_address2, announcer2),
            ],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );
}

// add multiple services, then query all but with a paging limit less than the number of services
// added
#[test]
fn paging_works() {
    let mut setup = TestSetup::new();
    let announcer1 = Addr::unchecked("wealthy_announcer_1");
    let announcer2 = Addr::unchecked("wealthy_announcer_2");
    let nym_address1 = NymAddress::new("nymAddress1");
    let nym_address2 = NymAddress::new("nymAddress2");

    // We announce the same address three times, but with different announcers
    setup.announce_net_req(nym_address1.clone(), announcer1.clone());
    setup.announce_net_req(nym_address1.clone(), announcer1.clone());
    setup.announce_net_req(nym_address2.clone(), announcer1.clone());
    setup.announce_net_req(nym_address1.clone(), announcer2.clone());
    setup.announce_net_req(nym_address2.clone(), announcer2.clone());

    assert_eq!(
        setup.query_all_with_limit(Some(10), None),
        PagedServicesListResponse {
            services: vec![
                service_info(1, nym_address1.clone(), announcer1.clone()),
                service_info(2, nym_address1.clone(), announcer1.clone()),
                service_info(3, nym_address2.clone(), announcer1.clone()),
                service_info(4, nym_address1.clone(), announcer2.clone()),
                service_info(5, nym_address2.clone(), announcer2.clone()),
            ],
            per_page: 10,
            start_next_after: Some(5),
        }
    );

    assert_eq!(
        setup.query_all_with_limit(Some(3), None),
        PagedServicesListResponse {
            services: vec![
                service_info(1, nym_address1.clone(), announcer1.clone()),
                service_info(2, nym_address1.clone(), announcer1.clone()),
                service_info(3, nym_address2.clone(), announcer1.clone()),
            ],
            per_page: 3,
            start_next_after: Some(3),
        }
    );
    assert_eq!(
        setup.query_all_with_limit(Some(3), Some(3)),
        PagedServicesListResponse {
            services: vec![
                service_info(4, nym_address1.clone(), announcer2.clone()),
                service_info(5, nym_address2.clone(), announcer2.clone()),
            ],
            per_page: 3,
            start_next_after: Some(5),
        }
    );
}

#[test]
fn service_id_increases_for_new_services() {
    let mut setup = TestSetup::new();
    setup.announce_net_req(
        NymAddress::new("nymAddress1"),
        Addr::unchecked("announcer1"),
    );
    setup.announce_net_req(
        NymAddress::new("nymAddress2"),
        Addr::unchecked("announcer2"),
    );

    assert_eq!(
        setup
            .query_all()
            .services
            .iter()
            .map(|s| s.service_id)
            .collect::<Vec<_>>(),
        vec![1, 2],
    );
}

#[test]
fn service_id_is_not_resused_when_deleting_and_then_adding_a_new_service() {
    let mut setup = TestSetup::new();
    setup.announce_net_req(
        NymAddress::new("nymAddress1"),
        Addr::unchecked("announcer1"),
    );
    setup.announce_net_req(
        NymAddress::new("nymAddress2"),
        Addr::unchecked("announcer2"),
    );
    setup.announce_net_req(
        NymAddress::new("nymAddress3"),
        Addr::unchecked("announcer3"),
    );

    setup.delete(1, Addr::unchecked("announcer1"));
    setup.delete(3, Addr::unchecked("announcer3"));

    assert_eq!(
        setup.query_all().services,
        vec![service_info(
            2,
            NymAddress::new("nymAddress2"),
            Addr::unchecked("announcer2")
        )]
    );

    setup.announce_net_req(
        NymAddress::new("nymAddress4"),
        Addr::unchecked("announcer4"),
    );

    assert_eq!(
        setup.query_all().services,
        vec![
            service_info(
                2,
                NymAddress::new("nymAddress2"),
                Addr::unchecked("announcer2")
            ),
            service_info(
                4,
                NymAddress::new("nymAddress4"),
                Addr::unchecked("announcer4")
            )
        ]
    );
}
