// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Offline verification of the frozen sandbox proofs in [`crate::fixtures`].
//!
//! These establish that the geolocation contract's digest key is reachable by exactly the
//! same machinery the directory uses: the shared ICS23 verifier in `nym-contract-anchor`,
//! against a contract address and a digest key supplied as parameters. No network.

use crate::fixtures::{ProofFixture, live_membership_fixture, live_non_membership_fixture};
use crate::key::digest_state_key;
use nym_contract_anchor::anchor::helpers::proven_contract_digest;
use nym_contract_anchor::error::ProofError;
use nym_contract_anchor::proof::{
    ProvenPresence, verify_wasm_store_membership, verify_wasm_store_non_membership,
    verify_wasm_store_presence,
};
use nym_lthash::{DIGEST_LEN, LtHash16};

/// The key the client rebuilds locally must be the key the chain actually proved. If these
/// ever diverge, every verified read fails against a correct chain - or worse, a verifier
/// checks a proof for a key it did not intend to read.
#[test]
fn the_reconstructed_key_matches_the_proven_key() {
    let fixture = live_membership_fixture();

    assert_eq!(
        digest_state_key(&fixture.contract),
        fixture.res.proof.ops[0].key,
        "locally rebuilt digest key must equal the key the proof commits to"
    );
}

/// 5.4: the frozen membership proof verifies, a tampered value is rejected, and a wrong
/// `app_hash` is rejected.
#[test]
fn the_membership_fixture_verifies_and_rejects_tampering() {
    let ProofFixture {
        res, key, app_hash, ..
    } = live_membership_fixture();

    // the proven value is the accumulator itself, not its 32-byte collapse
    assert_eq!(res.response.len(), DIGEST_LEN);

    // positive: the key/value pair is committed under the trusted app_hash
    verify_wasm_store_membership(&res.proof.ops, app_hash.as_bytes(), &key, &res.response)
        .expect("the captured membership proof must verify");

    // and the presence discriminator agrees, by proof shape
    assert!(matches!(
        verify_wasm_store_presence(&res.proof.ops, app_hash.as_bytes(), &key, &res.response),
        Ok(ProvenPresence::Present)
    ));

    // negative: a single flipped bit in the accumulator no longer matches the commitment
    let mut tampered = res.response.clone();
    tampered[0] ^= 0x01;
    assert!(matches!(
        verify_wasm_store_membership(&res.proof.ops, app_hash.as_bytes(), &key, &tampered),
        Err(ProofError::IavlVerificationFailed)
    ));

    // negative: the right value under the wrong root is rejected at the store layer. This is
    // the check that stops a malicious RPC serving a self-consistent header/proof pair.
    let mut wrong_app_hash = app_hash.as_bytes().to_vec();
    wrong_app_hash[0] ^= 0xff;
    assert!(matches!(
        verify_wasm_store_membership(&res.proof.ops, &wrong_app_hash, &key, &res.response),
        Err(ProofError::StoreVerificationFailed)
    ));
}

/// The non-membership fixture proves *its own* key absent, and nothing else. A proof that
/// could be replayed to show an existing key absent would let a source hide entries.
#[test]
fn the_non_membership_fixture_verifies_and_does_not_prove_other_keys_absent() {
    let ProofFixture {
        contract,
        res,
        key,
        app_hash,
        ..
    } = live_non_membership_fixture();

    verify_wasm_store_non_membership(&res.proof.ops, app_hash.as_bytes(), &key)
        .expect("the captured non-membership proof must verify");

    assert!(matches!(
        verify_wasm_store_presence(&res.proof.ops, app_hash.as_bytes(), &key, &res.response),
        Ok(ProvenPresence::Absent)
    ));

    // the digest key DOES exist at this height, so this gap proof must not cover it
    assert!(matches!(
        verify_wasm_store_non_membership(
            &res.proof.ops,
            app_hash.as_bytes(),
            &digest_state_key(&contract)
        ),
        Err(ProofError::IavlVerificationFailed)
    ));

    let mut wrong_app_hash = app_hash.as_bytes().to_vec();
    wrong_app_hash[0] ^= 0xff;
    assert!(matches!(
        verify_wasm_store_non_membership(&res.proof.ops, &wrong_app_hash, &key),
        Err(ProofError::StoreVerificationFailed)
    ));
}

/// The digest decode path, run offline against the real captured accumulator: a proven
/// present digest item reconstructs an `LtHash16` from the full `DIGEST_LEN` bytes.
#[test]
fn a_proven_present_digest_item_decodes_to_the_committed_accumulator() {
    let ProofFixture {
        res,
        key,
        app_hash,
        height,
        ..
    } = live_membership_fixture();

    let expected = LtHash16::from_bytes(
        &<[u8; DIGEST_LEN]>::try_from(res.response.clone()).expect("fixture value is DIGEST_LEN"),
    );

    let trusted = proven_contract_digest(res, &app_hash, &key, height)
        .expect("the captured digest read must verify");

    assert_eq!(trusted.height, height);
    assert_eq!(trusted.accumulator, expected);
    // the contract had entries at this height, so this is emphatically not the default
    assert_ne!(trusted.accumulator, LtHash16::new());
}

/// 5.5: a contract only writes its digest item on the first entry mutation, so one with no
/// entries has no item at all. A verified *absence* must read as the empty accumulator -
/// mirroring the contract's own load-time default - not as a verification failure.
///
/// Sandbox's geolocation contract already has a digest item, so its absence cannot be
/// captured there. The fixture instead proves a key the contract never writes absent, and
/// the digest key is passed as the parameter it now is - which is exactly the shape of the
/// read against a contract that has not yet written one.
#[test]
fn a_proven_absent_digest_key_yields_the_empty_accumulator() {
    let ProofFixture {
        res,
        key,
        app_hash,
        height,
        ..
    } = live_non_membership_fixture();

    let trusted = proven_contract_digest(res, &app_hash, &key, height)
        .expect("a proven-absent digest item must resolve, not error");

    assert_eq!(trusted.height, height);
    assert_eq!(trusted.accumulator, LtHash16::new());
}
