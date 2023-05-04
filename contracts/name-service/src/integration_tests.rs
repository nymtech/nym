//! Integration tests using cw-multi-test.

use cosmwasm_std::Addr;
use nym_name_service_common::{
    response::{ConfigResponse, PagedNamesListResponse},
    NameEntry, NymAddress, NymName, RegisteredName,
};

use crate::{
    constants::NAME_DEFAULT_RETRIEVAL_LIMIT,
    error::NameServiceError,
    test_helpers::{fixture::name_entry, helpers::nyms, test_setup::TestSetup},
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
fn register_and_query_name() {
    let mut setup = TestSetup::new();
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![],
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: None,
        }
    );

    // Register a first name
    let owner = Addr::unchecked("owner");
    let name = NymName::new("steves-server").unwrap();
    let nym_address = NymAddress::new("nym-address");
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&owner), nyms(250));
    setup.register(name.clone(), nym_address.clone(), owner.clone());

    // Deposit is deposited to contract and deducted from owners's balance
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance(&owner), nyms(150));

    // We can query the full name list
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![NameEntry {
                name_id: 1,
                name: RegisteredName {
                    nym_address: nym_address.clone(),
                    name: name.clone(),
                    owner: owner.clone(),
                    block_height: 12345,
                    deposit: nyms(100),
                },
            }],
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(1),
        }
    );

    // ... and we can query by id
    assert_eq!(
        setup.query_id(1),
        NameEntry {
            name_id: 1,
            name: RegisteredName {
                nym_address: nym_address.clone(),
                name: name.clone(),
                owner: owner.clone(),
                block_height: 12345,
                deposit: nyms(100),
            },
        }
    );

    // Register a second name
    let owner2 = Addr::unchecked("owner2");
    let name2 = NymName::new("another_server").unwrap();
    let nym_address2 = NymAddress::new("nymAddress2");
    setup.register(name2.clone(), nym_address2.clone(), owner2.clone());

    assert_eq!(setup.contract_balance(), nyms(200));
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![
                name_entry(1, name, nym_address, owner),
                name_entry(2, name2, nym_address2, owner2)
            ],
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(2),
        }
    );
}

#[test]
fn cant_register_a_name_without_funds() {
    let mut setup = TestSetup::new();
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance("owner"), nyms(250));
    setup.register(
        NymName::new("my_name").unwrap(),
        NymAddress::new("nymAddress"),
        Addr::unchecked("owner"),
    );
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance("owner"), nyms(150));
    setup.register(
        NymName::new("my_name2").unwrap(),
        NymAddress::new("nymAddress"),
        Addr::unchecked("owner"),
    );
    assert_eq!(setup.contract_balance(), nyms(200));
    assert_eq!(setup.balance("owner"), nyms(50));
    let res = setup
        .try_register(
            NymName::new("my_name3").unwrap(),
            NymAddress::new("nymAddress"),
            Addr::unchecked("owner"),
        )
        .unwrap_err();
    assert_eq!(
        res.downcast::<cosmwasm_std::StdError>().unwrap(),
        cosmwasm_std::StdError::Overflow {
            source: cosmwasm_std::OverflowError::new(
                cosmwasm_std::OverflowOperation::Sub,
                "50",
                "100"
            )
        }
    );
}

#[test]
fn delete_name() {
    let mut setup = TestSetup::new();
    setup.register(
        NymName::new("my_name").unwrap(),
        NymAddress::new("nymAddress"),
        Addr::unchecked("owner"),
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

#[test]
fn only_owner_can_delete_name() {
    let mut setup = TestSetup::new();
    assert_eq!(setup.contract_balance(), nyms(0));
    setup.register(
        NymName::new("name").unwrap(),
        NymAddress::new("nymAddress"),
        Addr::unchecked("owner"),
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

#[test]
fn cant_delete_name_that_does_not_exist() {
    let mut setup = TestSetup::new();
    setup.register(
        NymName::new("foo").unwrap(),
        NymAddress::new("nymAddress"),
        Addr::unchecked("owner"),
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

#[test]
fn cant_register_the_same_name_multiple_times() {
    let mut setup = TestSetup::new();

    setup.register(
        NymName::new("name").unwrap(),
        NymAddress::new("nymAddress"),
        Addr::unchecked("owner"),
    );
    let resp = setup
        .try_register(
            NymName::new("name").unwrap(),
            NymAddress::new("nymAddress"),
            Addr::unchecked("owner"),
        )
        .unwrap_err();

    assert_eq!(
        resp.downcast::<NameServiceError>().unwrap(),
        NameServiceError::NameAlreadyRegistered {
            name: NymName::new("name").unwrap()
        }
    );
}

#[test]
fn can_register_multiple_names_for_the_same_nym_address() {
    let mut setup = TestSetup::new();
    let name1 = NymName::new("name1").unwrap();
    let name2 = NymName::new("name2").unwrap();
    let address = NymAddress::new("nymaddress");
    let owner = Addr::unchecked("owner");

    setup.register(name1.clone(), address.clone(), owner.clone());
    setup.register(name2.clone(), address.clone(), owner.clone());

    assert_eq!(
        setup.query_all().names,
        vec![
            name_entry(1, name1, address.clone(), owner.clone()),
            name_entry(2, name2, address, owner)
        ],
    );
}

#[test]
fn register_multiple_names_and_deleting_by_name() {
    let mut setup = TestSetup::new();
    let owner1 = Addr::unchecked("wealthy_owner_1");
    let owner2 = Addr::unchecked("wealthy_owner_2");
    let nym_address1 = NymAddress::new("nymaddress1");
    let nym_address2 = NymAddress::new("nymaddress2");
    let name1 = NymName::new("name1").unwrap();
    let name2 = NymName::new("name2").unwrap();
    let name3 = NymName::new("name3").unwrap();
    let name4 = NymName::new("name4").unwrap();
    let name5 = NymName::new("name5").unwrap();

    // We register the same address three times, but with different owners
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&owner1), nyms(1000));
    setup.register(name1.clone(), nym_address1.clone(), owner1.clone());
    setup.register(name2.clone(), nym_address1.clone(), owner1.clone());
    setup.register(name3.clone(), nym_address2.clone(), owner1.clone());
    setup.register(name4.clone(), nym_address1.clone(), owner2.clone());
    setup.register(name5.clone(), nym_address2.clone(), owner2.clone());

    assert_eq!(setup.contract_balance(), nyms(500));
    assert_eq!(setup.balance(&owner1), nyms(700));
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![
                name_entry(1, name1.clone(), nym_address1.clone(), owner1.clone()),
                name_entry(2, name2.clone(), nym_address1.clone(), owner1.clone()),
                name_entry(3, name3.clone(), nym_address2.clone(), owner1.clone()),
                name_entry(4, name4.clone(), nym_address1.clone(), owner2.clone()),
                name_entry(5, name5.clone(), nym_address2.clone(), owner2.clone()),
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
                name_entry(2, name2.clone(), nym_address1.clone(), owner1.clone()),
                name_entry(3, name3.clone(), nym_address2.clone(), owner1.clone()),
                name_entry(4, name4.clone(), nym_address1.clone(), owner2.clone()),
                name_entry(5, name5, nym_address2.clone(), owner2.clone()),
            ],
            per_page: NAME_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(5),
        }
    );
}

#[test]
fn check_paging() {
    let mut setup = TestSetup::new();
    let owner1 = Addr::unchecked("wealthy_owner_1");
    let owner2 = Addr::unchecked("wealthy_owner_2");
    let nym_address1 = NymAddress::new("nymAddress1");
    let nym_address2 = NymAddress::new("nymAddress2");
    let name1 = NymName::new("name1").unwrap();
    let name2 = NymName::new("name2").unwrap();
    let name3 = NymName::new("name3").unwrap();
    let name4 = NymName::new("name4").unwrap();
    let name5 = NymName::new("name5").unwrap();

    // We register the same address three times, but with different owners
    assert_eq!(setup.contract_balance(), nyms(0));
    assert_eq!(setup.balance(&owner1), nyms(1000));
    setup.register(name1.clone(), nym_address1.clone(), owner1.clone());
    setup.register(name2.clone(), nym_address1.clone(), owner1.clone());
    setup.register(name3.clone(), nym_address2.clone(), owner1.clone());
    setup.register(name4.clone(), nym_address1.clone(), owner2.clone());
    setup.register(name5.clone(), nym_address2.clone(), owner2.clone());

    assert_eq!(setup.contract_balance(), nyms(500));
    assert_eq!(setup.balance(&owner1), nyms(700));
    assert_eq!(
        setup.query_all(),
        PagedNamesListResponse {
            names: vec![
                name_entry(1, name1.clone(), nym_address1.clone(), owner1.clone()),
                name_entry(2, name2.clone(), nym_address1.clone(), owner1.clone()),
                name_entry(3, name3.clone(), nym_address2.clone(), owner1.clone()),
                name_entry(4, name4.clone(), nym_address1.clone(), owner2.clone()),
                name_entry(5, name5, nym_address2.clone(), owner2.clone()),
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
                name_entry(2, name2, nym_address1.clone(), owner1.clone()),
                name_entry(3, name3.clone(), nym_address2.clone(), owner1.clone()),
            ],
            per_page: 2,
            start_next_after: Some(3),
        }
    );

    assert_eq!(
        setup.query_all_with_limit(Some(2), Some(2)),
        PagedNamesListResponse {
            names: vec![
                name_entry(3, name3, nym_address2, owner1),
                name_entry(4, name4, nym_address1, owner2),
            ],
            per_page: 2,
            start_next_after: Some(4),
        }
    );
}

#[test]
fn name_id_is_not_resused_when_deleting_and_then_adding_a_new_names() {
    let mut setup = TestSetup::new();
    setup.register(
        NymName::new("myname1").unwrap(),
        NymAddress::new("nymAddress1"),
        Addr::unchecked("owner1"),
    );
    setup.register(
        NymName::new("myname2").unwrap(),
        NymAddress::new("nymAddress2"),
        Addr::unchecked("owner2"),
    );
    setup.register(
        NymName::new("myname3").unwrap(),
        NymAddress::new("nymAddress3"),
        Addr::unchecked("owner3"),
    );

    setup.delete(1, Addr::unchecked("owner1"));
    setup.delete(3, Addr::unchecked("owner3"));

    assert_eq!(
        setup.query_all().names,
        vec![name_entry(
            2,
            NymName::new("myname2").unwrap(),
            NymAddress::new("nymAddress2"),
            Addr::unchecked("owner2")
        )]
    );

    setup.register(
        NymName::new("myname4").unwrap(),
        NymAddress::new("nymAddress4"),
        Addr::unchecked("owner4"),
    );

    assert_eq!(
        setup.query_all().names,
        vec![
            name_entry(
                2,
                NymName::new("myname2").unwrap(),
                NymAddress::new("nymAddress2"),
                Addr::unchecked("owner2")
            ),
            name_entry(
                4,
                NymName::new("myname4").unwrap(),
                NymAddress::new("nymAddress4"),
                Addr::unchecked("owner4")
            )
        ]
    );
}
