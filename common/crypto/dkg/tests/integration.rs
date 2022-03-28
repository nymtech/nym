// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::{G1Projective, Scalar};
use dkg::bte::proof_chunking::ProofOfChunking;
use dkg::bte::proof_sharing::ProofOfSecretSharing;
use dkg::bte::{decrypt_share, encrypt_shares, keygen, proof_chunking, proof_sharing, setup, Tau};
use dkg::interpolation::polynomial::Polynomial;
use rand_core::SeedableRng;

#[test]
fn single_sender() {
    // makes it easier to understand than `full_threshold_secret_sharing`
    // and is a good stepping stone, because its everything each node will have to perform (from one point of view)

    let dummy_seed = [42u8; 32];
    let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

    // the simplest possible case
    let nodes = 3;
    let threshold = 2;

    // the indices are going to get assigned externally, so for test sake, use non-consecutive ones
    let node_indices = vec![1u64, 2, 4];
    let params = setup();
    let mut keys = (0..nodes)
        .map(|_| keygen(&params, &mut rng))
        .collect::<Vec<_>>();

    let polynomial = Polynomial::new_random(&mut rng, threshold);
    let mut shares = node_indices
        .iter()
        .map(|&node_index| polynomial.evaluate(&Scalar::from(node_index)).into())
        .collect::<Vec<_>>();

    // start off in a defined epoch (i.e. not root);
    let epoch = Tau::new(2);

    let _self_keys = keys.pop().unwrap();
    let _self_share = shares.pop().unwrap();

    // TODO: HERE BE SERIALIZATION / DESERIALIZATION THAT'S NOT IMPLEMENTED YET
    // verify remote proofs of key possession
    for key in keys.iter() {
        assert!(key.1.verify());
    }

    let remote_public_keys = keys
        .iter()
        .map(|key| key.1.public_key().clone())
        .collect::<Vec<_>>();

    // note that we skip the first key since we don't have to encrypt share that we are generating for ourselves
    let remote_share_key_pairs = shares
        .iter()
        .zip(keys.iter())
        .map(|(share, keys)| (share, keys.1.public_key()))
        .collect::<Vec<_>>();

    let (ciphertext, hazmat) = encrypt_shares(&remote_share_key_pairs, &epoch, &params, &mut rng);

    let instance = proof_chunking::Instance::new(&remote_public_keys, &ciphertext);
    let proof_of_chunking = ProofOfChunking::construct(&mut rng, instance, hazmat.r(), &shares)
        .expect("failed to construct proof of chunking");

    // TODO: ask @AP or @AR whether this is the correct approach for combining those
    let combined_ciphertexts = ciphertext.combine_ciphertexts();
    let combined_r = hazmat.combine_rs();
    let combined_rr = ciphertext.combine_rs();

    let public_coefficients = polynomial.public_coefficients();
    let instance = proof_sharing::Instance::new(
        &remote_public_keys,
        &public_coefficients,
        &combined_rr,
        &combined_ciphertexts,
    );
    let proof_of_sharing =
        ProofOfSecretSharing::construct(&mut rng, instance, &combined_r, &shares)
            .expect("failed to construct proof of secret sharing");

    // TODO: HERE BE SERIALIZATION / DESERIALIZATION THAT'S NOT IMPLEMENTED YET
    // all other parties will have to:
    // - verify integrity of the ciphertext
    // - verify proof of chunking
    // - verify proof of sharing
    // - actually decrypt the shares
    assert!(ciphertext.verify_integrity(&params, &epoch));

    let instance = proof_chunking::Instance::new(&remote_public_keys, &ciphertext);
    assert!(proof_of_chunking.verify(instance));

    // TODO: verify proof of sharing
    let instance = proof_sharing::Instance::new(
        &remote_public_keys,
        &public_coefficients,
        &combined_rr,
        &combined_ciphertexts,
    );
    assert!(proof_of_sharing.verify(instance));

    // well, technically remote nodes are not going to know the actual plaintext share
    // but we might as well cheat a bit to verify correctness here
    for (i, (ref mut dk, _)) in keys.iter_mut().enumerate() {
        dk.try_update_to(&epoch, &params, &mut rng).unwrap();
        let recovered = decrypt_share(dk, i, &ciphertext, &epoch, None).unwrap();
        assert_eq!(shares[i], recovered)
    }
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
