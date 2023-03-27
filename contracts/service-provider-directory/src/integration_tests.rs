//! Integration tests using cw-multi-test.

use cosmwasm_std::{Addr, Coin};

use crate::{
    error::ContractError,
    msg::{ConfigResponse, ServiceInfo, ServicesListResponse},
    state::{NymAddress, Service, ServiceType},
    test_helpers::{helpers::nyms, test_setup::TestSetup},
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
            updater_role: Addr::unchecked("updater"),
            admin: Addr::unchecked("admin")
        }
    );
}

#[test]
fn announce_and_query_service() {
    let mut setup = TestSetup::new();
    assert_eq!(setup.query_all(), ServicesListResponse { services: vec![] });

    let admin_balance = setup.balance("admin").unwrap();
    let owner_balance = setup.balance("owner").unwrap();
    dbg!(&admin_balance);
    dbg!(&owner_balance);

    let owner = Addr::unchecked("owner");
    let nym_address = NymAddress::new("nymAddress");
    setup
        .announce_network_requester(nym_address.clone(), owner.clone())
        .unwrap();

    assert_eq!(
        setup.query_all(),
        ServicesListResponse {
            services: vec![ServiceInfo {
                service_id: 1,
                service: Service {
                    nym_address: nym_address.clone(),
                    service_type: ServiceType::NetworkRequester,
                    owner: owner.clone(),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            }]
        }
    );

    assert_eq!(
        setup.query_id(1),
        ServiceInfo {
            service_id: 1,
            service: Service {
                nym_address: nym_address.clone(),
                service_type: ServiceType::NetworkRequester,
                owner: owner.clone(),
                block_height: 12345,
                deposit: nyms(100),
            },
        }
    );

    let owner2 = Addr::unchecked("owner2");
    let nym_address2 = NymAddress::new("nymAddress2");
    setup
        .announce_network_requester(nym_address2.clone(), owner2.clone())
        .unwrap();

    assert_eq!(
        setup.query_all(),
        ServicesListResponse {
            services: vec![
                ServiceInfo {
                    service_id: 1,
                    service: Service {
                        nym_address,
                        service_type: ServiceType::NetworkRequester,
                        owner,
                        block_height: 12345,
                        deposit: nyms(100),
                    },
                },
                ServiceInfo {
                    service_id: 2,
                    service: Service {
                        nym_address: nym_address2,
                        service_type: ServiceType::NetworkRequester,
                        owner: owner2,
                        block_height: 12345,
                        deposit: nyms(100),
                    },
                }
            ]
        }
    );
}

#[test]
fn delete_service() {
    let mut setup = TestSetup::new();
    setup
        .announce_network_requester(NymAddress::new("nymAddress"), Addr::unchecked("owner"))
        .unwrap();
    assert!(!setup.query_all().services.is_empty());
    setup.delete(1, Addr::unchecked("owner")).unwrap();
    assert!(setup.query_all().services.is_empty());
}

#[test]
fn only_owner_can_delete_service() {
    let mut setup = TestSetup::new();
    setup
        .announce_network_requester(NymAddress::new("nymAddress"), Addr::unchecked("owner"))
        .unwrap();
    assert!(!setup.query_all().services.is_empty());

    let delete_resp: ContractError = setup
        .delete(1, Addr::unchecked("not_owner"))
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(
        delete_resp,
        ContractError::Unauthorized {
            sender: Addr::unchecked("not_owner")
        }
    );
}

#[test]
fn cant_delete_service_that_does_not_exist() {
    let mut setup = TestSetup::new();
    setup
        .announce_network_requester(NymAddress::new("nymAddress"), Addr::unchecked("owner"))
        .unwrap();
    assert!(!setup.query_all().services.is_empty());

    let delete_resp: ContractError = setup
        .delete(0, Addr::unchecked("owner"))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(delete_resp, ContractError::NotFound { service_id: 0 });

    let delete_resp: ContractError = setup
        .delete(2, Addr::unchecked("owner"))
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(delete_resp, ContractError::NotFound { service_id: 2 });

    assert!(!setup.query_all().services.is_empty());
    setup.delete(1, Addr::unchecked("owner")).unwrap();
    assert!(setup.query_all().services.is_empty());
}

#[test]
fn service_id_increases_for_new_services() {
    let mut setup = TestSetup::new();
    setup
        .announce_network_requester(NymAddress::new("nymAddress1"), Addr::unchecked("owner1"))
        .unwrap();
    setup
        .announce_network_requester(NymAddress::new("nymAddress2"), Addr::unchecked("owner2"))
        .unwrap();

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
    setup
        .announce_network_requester(NymAddress::new("nymAddress1"), Addr::unchecked("owner1"))
        .unwrap();
    setup
        .announce_network_requester(NymAddress::new("nymAddress2"), Addr::unchecked("owner2"))
        .unwrap();
    setup
        .announce_network_requester(NymAddress::new("nymAddress3"), Addr::unchecked("owner3"))
        .unwrap();

    setup.delete(1, Addr::unchecked("owner1")).unwrap();
    setup.delete(3, Addr::unchecked("owner3")).unwrap();

    assert_eq!(
        setup.query_all().services,
        vec![ServiceInfo {
            service_id: 2,
            service: Service {
                nym_address: NymAddress::new("nymAddress2"),
                service_type: ServiceType::NetworkRequester,
                owner: Addr::unchecked("owner2"),
                block_height: 12345,
                deposit: nyms(100),
            },
        }]
    );

    setup
        .announce_network_requester(NymAddress::new("nymAddress4"), Addr::unchecked("owner4"))
        .unwrap();

    assert_eq!(
        setup.query_all().services,
        vec![
            ServiceInfo {
                service_id: 2,
                service: Service {
                    nym_address: NymAddress::new("nymAddress2"),
                    service_type: ServiceType::NetworkRequester,
                    owner: Addr::unchecked("owner2"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            },
            ServiceInfo {
                service_id: 4,
                service: Service {
                    nym_address: NymAddress::new("nymAddress4"),
                    service_type: ServiceType::NetworkRequester,
                    owner: Addr::unchecked("owner4"),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            }
        ]
    );
}
