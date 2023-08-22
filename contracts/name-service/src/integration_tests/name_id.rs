use cosmwasm_std::Addr;
use nym_name_service_common::NymName;

use crate::test_helpers::{fixture::new_name, helpers::nyms};

use super::test_setup::TestSetup;

#[test]
fn name_id_is_not_resused_when_deleting_and_then_adding_a_new_names() {
    let mut setup = TestSetup::new();
    setup.sign_and_register(
        &NymName::new("myname1").unwrap(),
        &Addr::unchecked("owner1"),
        &nyms(100),
    );
    let s2 = setup.sign_and_register(
        &NymName::new("myname2").unwrap(),
        &Addr::unchecked("owner2"),
        &nyms(100),
    );
    setup.sign_and_register(
        &NymName::new("myname3").unwrap(),
        &Addr::unchecked("owner3"),
        &nyms(100),
    );

    setup.delete(1, Addr::unchecked("owner1"));
    setup.delete(3, Addr::unchecked("owner3"));

    assert_eq!(
        setup.query_all().names,
        vec![new_name(
            2,
            &NymName::new("myname2").unwrap(),
            s2.address(),
            &Addr::unchecked("owner2"),
            s2.identity_key(),
        )]
    );

    let s4 = setup.sign_and_register(
        &NymName::new("myname4").unwrap(),
        &Addr::unchecked("owner4"),
        &nyms(100),
    );

    assert_eq!(
        setup.query_all().names,
        vec![
            new_name(
                2,
                &NymName::new("myname2").unwrap(),
                s2.address(),
                &Addr::unchecked("owner2"),
                s2.identity_key(),
            ),
            new_name(
                4,
                &NymName::new("myname4").unwrap(),
                s4.address(),
                &Addr::unchecked("owner4"),
                s4.identity_key(),
            )
        ]
    );
}
