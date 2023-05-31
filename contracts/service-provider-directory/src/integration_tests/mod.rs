//! Integration tests using cw-multi-test.

use cosmwasm_std::Addr;
use nym_service_provider_directory_common::{
    response::{ConfigResponse, PagedServicesListResponse},
    NymAddress, Service, ServiceDetails, ServiceType,
};

use crate::{
    constants::SERVICE_DEFAULT_RETRIEVAL_LIMIT,
    error::ContractError,
    test_helpers::{fixture::new_service, helpers::nyms},
};

use test_setup::TestSetup;

mod announcing;
mod deleting;
mod test_service;
mod test_setup;

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

// add multiple services, then query all but with a paging limit less than the number of services
// added
#[test]
fn paging_works() {
    let mut setup = TestSetup::new();
    let announcer1 = Addr::unchecked("wealthy_announcer_1");
    let announcer2 = Addr::unchecked("wealthy_announcer_2");
    let nym_address1 = NymAddress::new("nymAddress1");
    let nym_address2 = NymAddress::new("nymAddress2");
    let deposit = nyms(100);

    // We announce the same address three times, but with different announcers
    let s1 = setup.sign_and_announce_net_req(&nym_address1, &announcer1, &deposit);
    let s2 = setup.sign_and_announce_net_req(&nym_address1, &announcer1, &deposit);
    let s3 = setup.sign_and_announce_net_req(&nym_address2, &announcer1, &deposit);
    let s4 = setup.sign_and_announce_net_req(&nym_address1, &announcer2, &deposit);
    let s5 = setup.sign_and_announce_net_req(&nym_address2, &announcer2, &deposit);

    assert_eq!(
        setup.query_all_with_limit(Some(10), None),
        PagedServicesListResponse {
            services: vec![
                new_service(1, &nym_address1, &announcer1, s1.identity_key()),
                new_service(2, &nym_address1, &announcer1, s2.identity_key()),
                new_service(3, &nym_address2, &announcer1, s3.identity_key()),
                new_service(4, &nym_address1, &announcer2, s4.identity_key()),
                new_service(5, &nym_address2, &announcer2, s5.identity_key()),
            ],
            per_page: 10,
            start_next_after: Some(5),
        }
    );

    assert_eq!(
        setup.query_all_with_limit(Some(3), None),
        PagedServicesListResponse {
            services: vec![
                new_service(1, &nym_address1, &announcer1, s1.identity_key()),
                new_service(2, &nym_address1, &announcer1, s2.identity_key()),
                new_service(3, &nym_address2, &announcer1, s3.identity_key()),
            ],
            per_page: 3,
            start_next_after: Some(3),
        }
    );
    assert_eq!(
        setup.query_all_with_limit(Some(3), Some(3)),
        PagedServicesListResponse {
            services: vec![
                new_service(4, &nym_address1, &announcer2, s4.identity_key()),
                new_service(5, &nym_address2, &announcer2, s5.identity_key()),
            ],
            per_page: 3,
            start_next_after: Some(5),
        }
    );
}

#[test]
fn service_id_increases_for_new_services() {
    let mut setup = TestSetup::new();
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress1"),
        &Addr::unchecked("announcer1"),
        &nyms(100),
    );
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress2"),
        &Addr::unchecked("announcer2"),
        &nyms(100),
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
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress1"),
        &Addr::unchecked("announcer1"),
        &nyms(100),
    );
    let s2 = setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress2"),
        &Addr::unchecked("announcer2"),
        &nyms(100),
    );
    setup.sign_and_announce_net_req(
        &NymAddress::new("nymAddress3"),
        &Addr::unchecked("announcer3"),
        &nyms(100),
    );

    setup.delete(1, &Addr::unchecked("announcer1"));
    setup.delete(3, &Addr::unchecked("announcer3"));

    assert_eq!(
        setup.query_all().services,
        vec![new_service(
            2,
            &NymAddress::new("nymAddress2"),
            &Addr::unchecked("announcer2"),
            s2.identity_key(),
        )]
    );

    let s4 = setup.new_signed_service(
        &NymAddress::new("nymAddress4"),
        &Addr::unchecked("announcer4"),
        &nyms(100),
    );
    setup.announce_net_req(&s4, &Addr::unchecked("announcer4"));

    assert_eq!(
        setup.query_all().services,
        vec![
            new_service(
                2,
                &NymAddress::new("nymAddress2"),
                &Addr::unchecked("announcer2"),
                s2.identity_key(),
            ),
            new_service(
                4,
                &NymAddress::new("nymAddress4"),
                &Addr::unchecked("announcer4"),
                s4.identity_key(),
            )
        ]
    );
}
