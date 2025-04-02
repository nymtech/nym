// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::ecash::tests::{voucher_fixture, TestFixture};
use nym_api_requests::ecash::models::CommitedDeposit;

#[tokio::test]
async fn issued_ticketbooks_for() {
    let deposit_id1 = 123;
    let deposit_id2 = 321;

    let voucher1 = voucher_fixture(Some(deposit_id1));
    let voucher2 = voucher_fixture(Some(deposit_id2));
    let expiration_date = voucher1.expiration_date();

    let signing_data1 = voucher1.prepare_for_signing();
    let request1 = voucher1.create_blind_sign_request_body(&signing_data1);

    let signing_data2 = voucher2.prepare_for_signing();
    let request2 = voucher2.create_blind_sign_request_body(&signing_data2);

    let test_fixture = TestFixture::new().await;
    test_fixture.add_chain_deposit(&voucher1);
    test_fixture.add_chain_deposit(&voucher2);

    // no ticketbooks issued yet
    let response = test_fixture
        .issued_ticketbooks_for_unchecked(expiration_date)
        .await;
    assert!(response.body.deposits.is_empty());
    assert!(response.body.merkle_root.is_none());

    test_fixture.issue_ticketbook(request1.clone()).await;
    let response1 = test_fixture
        .issued_ticketbooks_for_unchecked(expiration_date)
        .await;
    assert_eq!(
        response1.body.deposits,
        vec![CommitedDeposit {
            deposit_id: request1.deposit_id,
            merkle_index: 0,
        }]
    );
    assert!(response1.body.merkle_root.is_some());

    test_fixture.issue_ticketbook(request2.clone()).await;
    let response2 = test_fixture
        .issued_ticketbooks_for_unchecked(expiration_date)
        .await;
    let mut got_sorted = response2.body.deposits.clone();
    got_sorted.sort_by_key(|d| d.merkle_index);
    assert_eq!(
        got_sorted,
        vec![
            CommitedDeposit {
                deposit_id: request1.deposit_id,
                merkle_index: 0,
            },
            CommitedDeposit {
                deposit_id: request2.deposit_id,
                merkle_index: 1,
            }
        ]
    );
    assert!(response2.body.merkle_root.is_some());
    assert_ne!(response1.body.merkle_root, response2.body.merkle_root);
}

#[tokio::test]
async fn issued_ticketbooks_challenge_commitment() {
    let deposit_id1 = 123;
    let deposit_id2 = 321;

    let voucher1 = voucher_fixture(Some(deposit_id1));
    let voucher2 = voucher_fixture(Some(deposit_id2));
    let expiration_date = voucher1.expiration_date();

    let signing_data1 = voucher1.prepare_for_signing();
    let request1 = voucher1.create_blind_sign_request_body(&signing_data1);

    let signing_data2 = voucher2.prepare_for_signing();
    let request2 = voucher2.create_blind_sign_request_body(&signing_data2);

    let test_fixture = TestFixture::new().await;
    test_fixture.add_chain_deposit(&voucher1);
    test_fixture.add_chain_deposit(&voucher2);

    // empty challenge
    let response = test_fixture
        .issued_ticketbooks_challenge_commitment(expiration_date, vec![])
        .await;
    assert_eq!(
        response.text(),
        EcashError::MerkleProofGenerationFailure.to_string()
    );

    // // challenge for what we haven't issued
    let response = test_fixture
        .issued_ticketbooks_challenge_commitment(expiration_date, vec![deposit_id1])
        .await;
    assert_eq!(
        response.text(),
        EcashError::UnavailableTicketbook {
            deposit_id: deposit_id1
        }
        .to_string()
    );

    let _ = test_fixture.issue_ticketbook(request1.clone()).await;
    let response = test_fixture
        .issued_ticketbooks_challenge_commitment_unchecked(expiration_date, vec![deposit_id1])
        .await;
    assert_eq!(response.body.expiration_date, expiration_date);
    assert_eq!(response.body.merkle_proof.total_leaves(), 1);

    let _ = test_fixture.issue_ticketbook(request2.clone()).await;
    // proof for the old deposit
    let response = test_fixture
        .issued_ticketbooks_challenge_commitment_unchecked(expiration_date, vec![deposit_id1])
        .await;
    assert_eq!(response.body.expiration_date, expiration_date);
    assert_eq!(response.body.merkle_proof.total_leaves(), 2);

    // proof for new deposit
    let response = test_fixture
        .issued_ticketbooks_challenge_commitment_unchecked(expiration_date, vec![deposit_id2])
        .await;
    assert_eq!(response.body.expiration_date, expiration_date);
    assert_eq!(response.body.merkle_proof.total_leaves(), 2);

    // proof for BOTH deposits
    let response = test_fixture
        .issued_ticketbooks_challenge_commitment_unchecked(
            expiration_date,
            vec![deposit_id1, deposit_id2],
        )
        .await;
    assert_eq!(response.body.expiration_date, expiration_date);
    assert_eq!(response.body.merkle_proof.total_leaves(), 2);
}
