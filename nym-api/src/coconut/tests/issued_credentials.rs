// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::tests::{voucher_request_fixture, TestFixture};
use cosmwasm_std::coin;
use nym_api_requests::coconut::models::{
    EpochCredentialsResponse, IssuedCredentialResponse, IssuedCredentialsResponse, Pagination,
};
use nym_api_requests::coconut::CredentialsRequestBody;
use nym_coconut::Base58;
use nym_validator_client::nym_api::routes::{API_VERSION, BANDWIDTH, COCONUT_ROUTES};
use rocket::http::Status;
use std::collections::BTreeMap;

#[tokio::test]
async fn epoch_credentials() {
    let route_epoch1 = format!("/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/epoch-credentials/1");
    let route_epoch2 = format!("/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/epoch-credentials/2");
    let route_epoch42 = format!("/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/epoch-credentials/42");

    let test_fixture = TestFixture::new().await;

    // initially we expect 0 issued
    let response = test_fixture.rocket.get(&route_epoch1).dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let parsed_response: EpochCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

    assert_eq!(parsed_response.epoch_id, 1);
    assert_eq!(parsed_response.total_issued, 0);
    assert_eq!(parsed_response.first_epoch_credential_id, None);

    // get credential
    test_fixture.issue_dummy_credential().await;

    // now there should be one
    let response = test_fixture.rocket.get(&route_epoch1).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: EpochCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

    assert_eq!(parsed_response.epoch_id, 1);
    assert_eq!(parsed_response.total_issued, 1);
    assert_eq!(parsed_response.first_epoch_credential_id, Some(1));

    // and another
    test_fixture.issue_dummy_credential().await;

    let response = test_fixture.rocket.get(&route_epoch1).dispatch().await;

    assert_eq!(response.status(), Status::Ok);
    let parsed_response: EpochCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

    // note that first epoch credential didn't change
    assert_eq!(parsed_response.epoch_id, 1);
    assert_eq!(parsed_response.total_issued, 2);
    assert_eq!(parsed_response.first_epoch_credential_id, Some(1));

    test_fixture.set_epoch(2);

    let response = test_fixture.rocket.get(&route_epoch2).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: EpochCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

    // note the epoch change
    assert_eq!(parsed_response.epoch_id, 2);
    assert_eq!(parsed_response.total_issued, 0);
    assert_eq!(parsed_response.first_epoch_credential_id, None);

    test_fixture.issue_dummy_credential().await;

    let response = test_fixture.rocket.get(&route_epoch2).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: EpochCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();

    // note the epoch change
    assert_eq!(parsed_response.epoch_id, 2);
    assert_eq!(parsed_response.total_issued, 1);
    assert_eq!(parsed_response.first_epoch_credential_id, Some(3));

    // random epoch in the future
    let response = test_fixture.rocket.get(&route_epoch42).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: EpochCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(parsed_response.epoch_id, 42);
    assert_eq!(parsed_response.total_issued, 0);
    assert_eq!(parsed_response.first_epoch_credential_id, None);
}

#[tokio::test]
async fn issued_credential() {
    fn route(id: i64) -> String {
        format!("/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/issued-credential/{id}")
    }

    // let test_fixture = TestFixture::new()
    let hash1 = "6B27412050B823E58BB38447D7870BBC8CBE3C51C905BEA89D459ACCDA80A00E".to_string();
    let hash2 = "9F4DF28B36189B4410BC23D97FD757FC74B919122E80534CC2CA6F3D646F6518".to_string();

    let (voucher1, req1) = voucher_request_fixture(coin(1234, "unym"), Some(hash1.clone()));
    let (voucher2, req2) = voucher_request_fixture(coin(1234, "unym"), Some(hash2.clone()));

    let test_fixture = TestFixture::new().await;
    test_fixture.add_deposit_tx(&voucher1);
    test_fixture.add_deposit_tx(&voucher2);

    // random credential that was never issued
    let response = test_fixture.rocket.get(route(42)).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: IssuedCredentialResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert!(parsed_response.credential.is_none());

    let cred1 = test_fixture.issue_credential(req1).await;

    test_fixture.set_epoch(3);
    let cred2 = test_fixture.issue_credential(req2).await;

    let response = test_fixture.rocket.get(route(1)).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: IssuedCredentialResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let issued1 = parsed_response.credential.unwrap();

    let response = test_fixture.rocket.get(route(2)).dispatch().await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: IssuedCredentialResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    let issued2 = parsed_response.credential.unwrap();

    // TODO: currently we have no signature checks
    assert_eq!(1, issued1.credential.id);
    assert_eq!(1, issued1.credential.epoch_id);
    assert_eq!(voucher1.tx_hash(), issued1.credential.tx_hash);
    assert_eq!(
        cred1.to_bytes(),
        issued1.credential.blinded_partial_credential.to_bytes()
    );
    let cms: Vec<_> = voucher1
        .blind_sign_request()
        .get_private_attributes_pedersen_commitments()
        .iter()
        .map(|c| c.to_bs58())
        .collect();
    assert_eq!(
        cms,
        issued1
            .credential
            .bs58_encoded_private_attributes_commitments
    );
    assert_eq!(
        voucher1.get_public_attributes_plain(),
        issued1.credential.public_attributes
    );

    assert_eq!(2, issued2.credential.id);
    assert_eq!(3, issued2.credential.epoch_id);
    assert_eq!(voucher2.tx_hash(), issued2.credential.tx_hash);
    assert_eq!(
        cred2.to_bytes(),
        issued2.credential.blinded_partial_credential.to_bytes()
    );
    let cms: Vec<_> = voucher2
        .blind_sign_request()
        .get_private_attributes_pedersen_commitments()
        .iter()
        .map(|c| c.to_bs58())
        .collect();
    assert_eq!(
        cms,
        issued2
            .credential
            .bs58_encoded_private_attributes_commitments
    );
    assert_eq!(
        voucher2.get_public_attributes_plain(),
        issued2.credential.public_attributes
    );
}

#[tokio::test]
async fn issued_credentials() {
    let route = format!("/{API_VERSION}/{COCONUT_ROUTES}/{BANDWIDTH}/issued-credentials");

    let test_fixture = TestFixture::new().await;

    // issue some credentials
    for _ in 0..20 {
        test_fixture.issue_dummy_credential().await;
    }

    let issued1 = test_fixture.issued_unchecked(1).await;
    let issued2 = test_fixture.issued_unchecked(2).await;
    let issued3 = test_fixture.issued_unchecked(3).await;
    let issued4 = test_fixture.issued_unchecked(4).await;
    let issued5 = test_fixture.issued_unchecked(5).await;
    let issued13 = test_fixture.issued_unchecked(13).await;

    let response = test_fixture
        .rocket
        .post(&route)
        .json(&CredentialsRequestBody {
            credential_ids: vec![5],
            pagination: None,
        })
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: IssuedCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(parsed_response.credentials[&5], issued5);
    assert!(parsed_response.credentials.get(&13).is_none());

    let response = test_fixture
        .rocket
        .post(&route)
        .json(&CredentialsRequestBody {
            credential_ids: vec![5, 13],
            pagination: None,
        })
        .dispatch()
        .await;
    assert_eq!(response.status(), Status::Ok);
    let parsed_response: IssuedCredentialsResponse =
        serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
    assert_eq!(parsed_response.credentials[&5], issued5);
    assert_eq!(parsed_response.credentials[&13], issued13);

    let response_paginated = test_fixture
        .rocket
        .post(&route)
        .json(&CredentialsRequestBody {
            credential_ids: vec![],
            pagination: Some(Pagination {
                last_key: None,
                limit: Some(2),
            }),
        })
        .dispatch()
        .await;
    assert_eq!(response_paginated.status(), Status::Ok);
    let parsed_response: IssuedCredentialsResponse =
        serde_json::from_str(&response_paginated.into_string().await.unwrap()).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert(1, issued1);
    expected.insert(2, issued2);
    assert_eq!(expected, parsed_response.credentials);

    let response_paginated = test_fixture
        .rocket
        .post(&route)
        .json(&CredentialsRequestBody {
            credential_ids: vec![],
            pagination: Some(Pagination {
                last_key: Some(2),
                limit: Some(3),
            }),
        })
        .dispatch()
        .await;
    assert_eq!(response_paginated.status(), Status::Ok);
    let parsed_response: IssuedCredentialsResponse =
        serde_json::from_str(&response_paginated.into_string().await.unwrap()).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert(3, issued3);
    expected.insert(4, issued4);
    expected.insert(5, issued5);
    assert_eq!(expected, parsed_response.credentials);
}
