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

use super::test_setup::TestSetup;

#[test]
fn basic_announce() {
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
    assert_eq!(setup.query_signing_nonce(announcer.to_string()), 0);

    let service = setup.new_service(&nym_address);
    let payload = setup.payload_to_sign(&announcer, &nyms(100), &service.service);
    let service = service.sign(payload);
    setup.announce_net_req(&service, &announcer);

    // Deposit is deposited to contract and deducted from announcers's balance
    assert_eq!(setup.contract_balance(), nyms(100));
    assert_eq!(setup.balance(&announcer), nyms(150));

    // The signing nonce has been incremented
    assert_eq!(setup.query_signing_nonce(announcer.to_string()), 1);

    // We can query the full service list
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![Service {
                service_id: 1,
                service: ServiceDetails {
                    nym_address: nym_address.clone(),
                    service_type: ServiceType::NetworkRequester,
                    identity_key: service.identity_key().to_string(),
                },
                announcer: announcer.clone(),
                block_height: 12345,
                deposit: nyms(100),
            }],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(1),
        }
    );

    // ... and we can query by id
    assert_eq!(
        setup.query_id(1),
        Service {
            service_id: 1,
            service: service.details().clone(),
            announcer: announcer.clone(),
            block_height: 12345,
            deposit: nyms(100),
        }
    );

    // Announce a second service
    let announcer2 = Addr::unchecked("announcer2");
    let nym_address2 = NymAddress::new("nymAddress2");
    let service2 = setup.new_signed_service(&nym_address2, &announcer2, &nyms(100));
    setup.announce_net_req(&service2, &announcer2);
    assert_eq!(setup.query_signing_nonce(announcer2.to_string()), 1);

    assert_eq!(setup.contract_balance(), nyms(200));
    assert_eq!(
        setup.query_all(),
        PagedServicesListResponse {
            services: vec![
                new_service(1, &nym_address, &announcer, service.identity_key()),
                new_service(2, &nym_address2, &announcer2, service2.identity_key())
            ],
            per_page: SERVICE_DEFAULT_RETRIEVAL_LIMIT as usize,
            start_next_after: Some(2),
        }
    );
}

#[test]
fn announce_fails_when_announcer_mismatch() {
    let mut setup = TestSetup::new();
    let announcer = Addr::unchecked("steve");
    let nym_address = NymAddress::new("foobar");
    let service = setup.new_signed_service(&nym_address, &announcer, &nyms(100));

    // A difference announcer tries to announce the service
    let announcer2 = Addr::unchecked("timmy");

    let resp: ContractError = setup
        .try_announce_net_req(&service, &announcer2)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(resp, ContractError::InvalidEd25519Signature);
}

#[test]
fn creating_two_services_in_a_row_without_announcing_fails() {
    let mut setup = TestSetup::new();
    let announcer = Addr::unchecked("wealthy_announcer_1");
    let nym_address = NymAddress::new("nymAddress");
    let deposit = nyms(100);

    // Signing two new services for the same announcer without announcing the first one will fail
    // since the nonce will not match
    let s1 = setup.new_signed_service(&nym_address, &announcer, &deposit);
    let s2 = setup.new_signed_service(&nym_address, &announcer, &deposit);

    // This will use the wrong nonce when verifying the signature
    // WIP(JON): appears broken!
    let resp: ContractError = setup
        .try_announce_net_req(&s1, &announcer)
        .unwrap_err()
        .downcast()
        .unwrap();
    assert_eq!(resp, ContractError::InvalidEd25519Signature,);
}

#[test]
fn signing_nonce_is_increased_when_announcing() {
    let mut setup = TestSetup::new();

    let announcer1 = Addr::unchecked("announcer1");
    let announcer2 = Addr::unchecked("announcer2");

    assert_eq!(setup.query_signing_nonce(announcer1.to_string()), 0);
    assert_eq!(setup.query_signing_nonce(announcer2.to_string()), 0);

    setup.sign_and_announce_net_req(&NymAddress::new("nymAddress1"), &announcer1, &nyms(100));

    assert_eq!(setup.query_signing_nonce(announcer1.to_string()), 1);
    assert_eq!(setup.query_signing_nonce(announcer2.to_string()), 0);

    setup.sign_and_announce_net_req(&NymAddress::new("nymAddress2"), &announcer2, &nyms(100));

    assert_eq!(setup.query_signing_nonce(announcer1.to_string()), 1);
    assert_eq!(setup.query_signing_nonce(announcer2.to_string()), 1);

    setup.sign_and_announce_net_req(&NymAddress::new("nymAddress3"), &announcer2, &nyms(100));

    assert_eq!(setup.query_signing_nonce(announcer1.to_string()), 1);
    assert_eq!(setup.query_signing_nonce(announcer2.to_string()), 2);
}
