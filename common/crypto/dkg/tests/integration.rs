// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use dkg::bte::{decrypt_share, keygen, setup, Epoch};
use dkg::interpolation::perform_lagrangian_interpolation_at_origin;
use dkg::{combine_shares, Dealing};
use rand_core::SeedableRng;
use std::collections::BTreeMap;

#[test]
fn single_sender() {
    // makes it easier to understand than `full_threshold_secret_sharing`
    // and is a good stepping stone, because its everything each node will have to perform (from one point of view)

    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    // the simplest possible case
    let threshold = 2;

    // the indices are going to get assigned externally, so for test sake, use non-consecutive ones
    let node_indices = vec![15u64, 248, 33521];

    let mut receivers = BTreeMap::new();
    let mut full_keys = Vec::new();
    for index in &node_indices {
        let (dk, pk) = keygen(&params, &mut rng);
        receivers.insert(*index, *pk.public_key());
        full_keys.push((dk, pk))
    }

    // start off in a defined epoch (i.e. not root);
    let epoch = Epoch::new(2);

    // TODO: HERE BE SERIALIZATION / DESERIALIZATION THAT'S NOT IMPLEMENTED YET
    // verify remote proofs of key possession
    for key in full_keys.iter() {
        assert!(key.1.verify());
    }

    let (dealing, dealer_share) = Dealing::create(
        &mut rng,
        &params,
        node_indices[0],
        threshold,
        epoch,
        &receivers,
    );
    dealing
        .verify(&params, epoch, threshold, &receivers)
        .unwrap();

    // make sure each share is actually decryptable (even though proofs say they must be, perform this sanity check)
    for (i, (ref mut dk, _)) in full_keys.iter_mut().enumerate() {
        dk.try_update_to(epoch, &params, &mut rng).unwrap();
        let _recovered = decrypt_share(dk, i, &dealing.ciphertexts, epoch, None).unwrap();
    }

    // and for good measure, check that the dealer's share matches decryption result
    let recovered_dealer =
        decrypt_share(&full_keys[0].0, 0, &dealing.ciphertexts, epoch, None).unwrap();
    assert_eq!(recovered_dealer, dealer_share.unwrap())
}

#[test]
fn full_threshold_secret_sharing() {
    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
    let params = setup();

    // the simplest possible case
    let threshold = 2;

    // the indices are going to get assigned externally, so for test sake, use non-consecutive ones
    let node_indices = vec![15u64, 248, 33521];

    let mut receivers = BTreeMap::new();
    let mut full_keys = Vec::new();
    for index in &node_indices {
        let (dk, pk) = keygen(&params, &mut rng);
        receivers.insert(*index, *pk.public_key());
        full_keys.push((dk, pk))
    }

    // start off in a defined epoch (i.e. not root);
    let epoch = Epoch::new(2);

    // TODO: HERE BE SERIALIZATION / DESERIALIZATION THAT'S NOT IMPLEMENTED YET
    // verify remote proofs of key possession
    for key in full_keys.iter() {
        assert!(key.1.verify());
    }

    let dealings = node_indices
        .iter()
        .map(|&dealer_index| {
            Dealing::create(
                &mut rng,
                &params,
                dealer_index,
                threshold,
                epoch,
                &receivers,
            )
            .0
        })
        .collect::<Vec<_>>();
    for dealing in &dealings {
        dealing
            .verify(&params, epoch, threshold, &receivers)
            .unwrap();
    }

    let mut derived_secrets = Vec::new();
    for (i, (ref mut dk, _)) in full_keys.iter_mut().enumerate() {
        dk.try_update_to(epoch, &params, &mut rng).unwrap();

        let shares = dealings
            .iter()
            .map(|dealing| decrypt_share(dk, i, &dealing.ciphertexts, epoch, None).unwrap())
            .collect();

        // we know dealer_share matches, but it would be inconvenient to try to put them in here,
        // so for ease of use (IN A TEST SETTING), just decrypt one's own share
        derived_secrets.push(combine_shares(shares))
    }

    // sanity check that the shares were combined correctly and if we take threshold number of them,
    // we end up with the same master secret, note: those are NEVER explicitly recovered in actual system
    // (remember threshold was 2)
    let master1 = perform_lagrangian_interpolation_at_origin(&[
        (Scalar::from(node_indices[0]), derived_secrets[0]),
        (Scalar::from(node_indices[1]), derived_secrets[1]),
    ])
    .unwrap();

    let master2 = perform_lagrangian_interpolation_at_origin(&[
        (Scalar::from(node_indices[1]), derived_secrets[1]),
        (Scalar::from(node_indices[2]), derived_secrets[2]),
    ])
    .unwrap();

    assert_eq!(master1, master2)
}
