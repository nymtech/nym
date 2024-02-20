// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::random_scalars_refs;
use crate::tests::helpers::tests::generate_dkg_keys;
use crate::{
    aggregate_verification_keys, setup, tests::helpers::*, ttp_keygen, verify_credential,
    CoconutError, VerificationKey,
};

#[test]
fn keygen() -> Result<(), CoconutError> {
    let params = setup(5)?;
    let node_indices = vec![15u64, 248, 33521];

    random_scalars_refs!(public_attributes, params, 2);

    // generate_keys
    let coconut_keypairs = ttp_keygen(&params, 2, 3)?;

    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key().clone())
        .collect();

    // aggregate verification keys
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&node_indices))?;

    // Generate cryptographic material to verify them
    let theta = theta_from_keys_and_attributes(
        &params,
        &coconut_keypairs,
        &node_indices,
        &public_attributes,
    )?;

    // Verify credentials
    assert!(verify_credential(
        &params,
        &verification_key,
        &theta,
        &public_attributes,
    ));

    Ok(())
}

#[test]
#[ignore] // expensive test
fn dkg() -> Result<(), CoconutError> {
    let params = setup(5)?;
    let node_indices = vec![15u64, 248, 33521];

    random_scalars_refs!(public_attributes, params, 2);

    // generate_keys
    let coconut_keypairs = generate_dkg_keys(5, &node_indices);

    let verification_keys: Vec<VerificationKey> = coconut_keypairs
        .iter()
        .map(|keypair| keypair.verification_key().clone())
        .collect();

    // aggregate verification keys
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&node_indices))?;

    // Generate cryptographic material to verify them
    let theta = theta_from_keys_and_attributes(
        &params,
        &coconut_keypairs,
        &node_indices,
        &public_attributes,
    )?;

    // Verify credentials
    assert!(verify_credential(
        &params,
        &verification_key,
        &theta,
        &public_attributes,
    ));

    Ok(())
}
