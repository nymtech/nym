use cosmwasm_std::Addr;
use nym_name_service_common::{response::PagedNamesListResponse, Address, NymName};
use rstest::rstest;

use crate::{
    constants::NAME_DEFAULT_RETRIEVAL_LIMIT,
    test_helpers::{fixture::new_name, helpers::nyms},
    NameServiceError,
};

use super::test_setup::TestSetup;

#[rstest::fixture]
fn setup() -> TestSetup {
    TestSetup::new()
}

#[rstest]
fn delete_name(mut setup: TestSetup) {
    setup.sign_and_register(
        &NymName::new("my_name").unwrap(),
        &Address::new("address"),
        &Addr::unchecked("owner"),
        &nyms(100),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance("owner"), nyms(150));
    assert!(!setup.query_all().names.is_empty());
    setup.delete(1, Addr::unchecked("owner"));

    // Deleting the name returns the deposit to the owner
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance("owner"), nyms(250));
    assert!(setup.query_all().names.is_empty());
}

#[rstest]
fn only_owner_can_delete_name(mut setup: TestSetup) {
    assert_eq!(setup.contract_balance(), nyms(0));
    setup.sign_and_register(
        &NymName::new("name").unwrap(),
        &Address::new("nymAddress"),
        &Addr::unchecked("owner"),
        &nyms(100),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert!(!setup.query_all().names.is_empty());

    let delete_resp = setup
        .try_delete(1, Addr::unchecked("not_owner"))
        .unwrap_err();

    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(
        delete_resp.downcast::<NameServiceError>().unwrap(),
        NameServiceError::Unauthorized {
            sender: Addr::unchecked("not_owner")
        }
    );
}

#[rstest]
fn cant_delete_name_that_does_not_exist(mut setup: TestSetup) {
    setup.sign_and_register(
        &NymName::new("foo").unwrap(),
        &Address::new("nymAddress"),
        &Addr::unchecked("owner"),
        &nyms(100),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert!(!setup.query_all().names.is_empty());

    let delete_resp = setup.try_delete(0, Addr::unchecked("owner")).unwrap_err();
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(
        delete_resp.downcast::<NameServiceError>().unwrap(),
        NameServiceError::NotFound { name_id: 0 }
    );

    let delete_resp = setup.try_delete(2, Addr::unchecked("owner")).unwrap_err();
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(
        delete_resp.downcast::<NameServiceError>().unwrap(),
        NameServiceError::NotFound { name_id: 2 }
    );

    assert!(!setup.query_all().names.is_empty());
    setup.delete(1, Addr::unchecked("owner"));
    assert_eq!(setup.contract_balance(), nyms(0));
    assert!(setup.query_all().names.is_empty());
}

#[rstest]
fn register_multiple_names_and_deleting_by_name(mut setup: TestSetup) {
    let owner1 = Addr::unchecked("wealthy_owner_1");
    let owner2 = Addr::unchecked("wealthy_owner_2");
    let address1 = Address::new("address1");
    let address2 = Address::new("address2");
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

    assert_eq!(setup.contract_balance(), nyms(400));
    assert_eq!(setup.balance(&owner1), nyms(800));
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![
                new_name(2, &name2, &address1, &owner1, s2.identity_key()),
                new_name(3, &name3, &address2, &owner1, s3.identity_key()),
                new_name(4, &name4, &address1, &owner2, s4.identity_key()),
                new_name(5, &name5, &address2, &owner2, s5.identity_key()),
            ],
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );
}
