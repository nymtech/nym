use cosmwasm_std::Addr;

use crate::{
    msg::{ConfigResponse, QueryMsg, ServiceInfo, ServicesListResponse},
    state::{NymAddress, Service, ServiceType},
    test_helpers::TestSetup, error::ContractError,
};

#[test]
fn instantiate_contract_with_helpers() {
    TestSetup::new();
}

#[test]
fn query_config() {
    let setup = TestSetup::new();
    let resp: ConfigResponse = setup.query(&QueryMsg::QueryConfig {});

    assert_eq!(
        resp,
        ConfigResponse {
            updater_role: Addr::unchecked("updater"),
            admin: Addr::unchecked("admin")
        }
    );
}

#[test]
fn announce_and_query_service() {
    let owner = Addr::unchecked("owner");
    let nym_address = NymAddress::new("nymAddress");
    let mut setup = TestSetup::new();
    setup
        .announce_network_requester(nym_address.clone(), owner.clone())
        .unwrap();

    assert_eq!(
        setup.query_all(),
        ServicesListResponse {
            services: vec![ServiceInfo {
                service_id: 1,
                service: Service {
                    nym_address,
                    service_type: ServiceType::NetworkRequester,
                    owner,
                    block_height: 12345,
                },
            }]
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
#[ignore]
fn delete_service_that_does_not_exist() {
    todo!();
}

#[test]
#[ignore]
fn service_id_increases_for_new_services() {
    todo!();
}

#[test]
#[ignore]
fn service_id_is_not_resused_when_deleting_and_then_adding_a_new_service() {
    todo!();
}
