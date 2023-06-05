use cosmwasm_std::Addr;
use nym_service_provider_directory_common::NymAddress;

use crate::test_helpers::{fixture::new_service, helpers::nyms};

use super::test_setup::TestSetup;

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
