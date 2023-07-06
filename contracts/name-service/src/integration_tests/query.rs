use cosmwasm_std::Addr;
use nym_name_service_common::{
    response::{ConfigResponse, PagedNamesListResponse},
    Address, NymName,
};

use crate::{
    constants::NAME_DEFAULT_RETRIEVAL_LIMIT,
    test_helpers::{fixture::new_name, helpers::nyms},
};

use super::test_setup::TestSetup;

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
fn check_paging() {
    let mut setup = TestSetup::new();
    let owner1 = Addr::unchecked("wealthy_owner_1");
    let owner2 = Addr::unchecked("wealthy_owner_2");
    let address1 = Address::new("nymAddress1");
    let address2 = Address::new("nymAddress2");
    let name1 = NymName::new("name1").unwrap();
    let name2 = NymName::new("name2").unwrap();
    let name3 = NymName::new("name3").unwrap();
    let name4 = NymName::new("name4").unwrap();
    let name5 = NymName::new("name5").unwrap();

    // We register the same address three times, but with different owners
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&owner1), nyms(1000));
    let s1 = setup.sign_and_register(&name1, &address1, &owner1, &nyms(100));
    let s2 = setup.sign_and_register(&name2, &address1, &owner1, &nyms(100));
    let s3 = setup.sign_and_register(&name3, &address2, &owner1, &nyms(100));
    let s4 = setup.sign_and_register(&name4, &address1, &owner2, &nyms(100));
    let s5 = setup.sign_and_register(&name5, &address2, &owner2, &nyms(100));

    assert_eq!(setup.contract_balance(), nyms(500));
    assert_eq!(setup.balance(&owner1), nyms(700));
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![
                new_name(1, &name1, &address1, &owner1, s1.identity_key()),
                new_name(2, &name2, &address1, &owner1, s2.identity_key()),
                new_name(3, &name3, &address2, &owner1, s3.identity_key()),
                new_name(4, &name4, &address1, &owner2, s4.identity_key()),
                new_name(5, &name5, &address2, &owner2, s5.identity_key()),
            ],
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );

    setup.delete_name(name1, owner1.clone());

    assert_eq!(
        setup.query_all_with_limit(Some(2), None),
        PagedNamesListResponse {
            names: vec![
                new_name(2, &name2, &address1, &owner1, s2.identity_key()),
                new_name(3, &name3, &address2, &owner1, s3.identity_key()),
            ],
            per_page: 2,
            start_next_after: Some(3),
        }
    );

    assert_eq!(
        setup.query_all_with_limit(Some(2), Some(2)),
        PagedNamesListResponse {
            names: vec![
                new_name(3, &name3, &address2, &owner1, s3.identity_key()),
                new_name(4, &name4, &address1, &owner2, s4.identity_key()),
            ],
            per_page: 2,
            start_next_after: Some(4),
        }
    );
}
