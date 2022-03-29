// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;
use dkg::bte::proof_chunking::ProofOfChunking;
use dkg::bte::proof_sharing::ProofOfSecretSharing;
use dkg::bte::{
    decrypt_share, encrypt_shares, keygen, proof_chunking, proof_sharing, setup, Ciphertexts,
    DecryptionKey, Params, PublicKey, Tau,
};
use dkg::interpolation::polynomial::Polynomial;
use dkg::{Dealing, Share, Threshold};
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
    let node_indices = vec![1u64, 2, 4];

    let mut receivers = BTreeMap::new();
    let mut full_keys = Vec::new();
    for index in &node_indices {
        let (dk, pk) = keygen(&params, &mut rng);
        receivers.insert(*index, *pk.public_key());
        full_keys.push((dk, pk))
    }

    // start off in a defined epoch (i.e. not root);
    let epoch = Tau::new(2);

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
        &epoch,
        &receivers,
    );

    // make sure each share is actually decryptable (even though proofs say they must be, perform this sanity check)
    for (i, (ref mut dk, _)) in full_keys.iter_mut().enumerate() {
        dk.try_update_to(&epoch, &params, &mut rng).unwrap();
        let _recovered = decrypt_share(dk, i, &dealing.ciphertexts, &epoch, None).unwrap();
    }

    // and for good measure, check that the dealer's share matches decryption result
    let recovered_dealer =
        decrypt_share(&full_keys[0].0, 0, &dealing.ciphertexts, &epoch, None).unwrap();
    assert_eq!(recovered_dealer, dealer_share.unwrap())
}

//
// #[test]
// fn full_threshold_secret_sharing() {
//     let dummy_seed = [42u8; 32];
//     let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
//
//     // the simplest possible case
//     let nodes = 3;
//     let threshold = 2;
//
//     // the indices are going to get assigned externally, so for test sake, use non-consecutive ones
//     let node_indices = vec![1u64, 2, 4];
//     let params = setup();
//     let mut keys = (0..nodes)
//         .map(|_| keygen(&params, &mut rng))
//         .collect::<Vec<_>>();
//     let polynomials = keys
//         .iter()
//         .map(|_| Polynomial::new_random(&mut rng, threshold))
//         .collect::<Vec<_>>();
//
//     // each node generates share for all other nodes (including itself)
//     let shares = polynomials
//         .iter()
//         .map(|poly| {
//             node_indices
//                 .iter()
//                 .map(|&node_index| poly.evaluate(&Scalar::from(node_index)))
//                 .collect::<Vec<_>>()
//         })
//         .collect::<Vec<_>>();
//
//     // poly0 => share1, share2, share4
//     // poly1 => share1, share2, share4
//     // poly2 => share1, share2, share4
//
//     // start off in a defined epoch (i.e. not root);
//     let epoch = Tau::new(2);
//
//     // TODO: HERE BE SERIALIZATION / DESERIALIZATION THAT'S NOT IMPLEMENTED YET
// }
