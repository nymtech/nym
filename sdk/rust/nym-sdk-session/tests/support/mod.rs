// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Local ecash fixtures for the signer-failure tests: a real 2-of-3 threshold
//! signer set generated in-process, used to fabricate valid ticketbooks and the
//! global signing data a healthy signer set would serve — no chain, no network,
//! no funds. Mirrors the real issuance path
//! (`nym-credentials::ecash::bandwidth::issuance`): withdrawal request →
//! per-authority blind signature → unblind → aggregate.

// Shared by several test binaries, each using a subset of these helpers.
#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

pub mod fake_dkg;
pub mod http_harness;

use nym_compact_ecash::scheme::keygen::{KeyPairAuth, SecretKeyAuth};
use nym_compact_ecash::tests::helpers::{
    generate_coin_indices_signatures, generate_expiration_date_signatures,
};
use nym_compact_ecash::{issue, ttp_keygen};
use nym_credentials::{AggregatedCoinIndicesSignatures, EpochVerificationKey, IssuedTicketBook};
use nym_credentials_interface::{
    aggregate_verification_keys, aggregate_wallets, ecash_parameters,
    generate_keypair_user_from_seed, issue_verify, withdrawal_request, AnnotatedCoinIndexSignature,
    AnnotatedExpirationDateSignature, Base58, PartialWallet, TicketType, VerificationKeyAuth,
};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::{ecash_default_expiration_date, Date, EcashTime};

/// The single ecash epoch all fixtures live in.
pub const EPOCH_ID: u64 = 1;

const AUTHORITIES: u64 = 3;
const THRESHOLD: u64 = 2;

/// A local threshold signer set plus the global data it would serve.
pub struct TestEcash {
    authorities: Vec<KeyPairAuth>,
    verification_keys: Vec<VerificationKeyAuth>,
    master_vk: VerificationKeyAuth,
    coin_index_signatures: Vec<AnnotatedCoinIndexSignature>,
}

impl TestEcash {
    pub fn new() -> Self {
        let authorities = ttp_keygen(THRESHOLD, AUTHORITIES).expect("ttp keygen");
        let verification_keys: Vec<VerificationKeyAuth> =
            authorities.iter().map(|kp| kp.verification_key()).collect();
        let indices: Vec<u64> = (1..=AUTHORITIES).collect();
        let master_vk = aggregate_verification_keys(&verification_keys, Some(&indices))
            .expect("aggregate verification keys");

        let secret_keys: Vec<&SecretKeyAuth> =
            authorities.iter().map(|kp| kp.secret_key()).collect();
        let coin_index_signatures = generate_coin_indices_signatures(
            ecash_parameters(),
            &secret_keys,
            &verification_keys,
            &master_vk,
            &indices,
        )
        .expect("coin index signatures")
        .into_iter()
        .enumerate()
        .map(|(index, signature)| AnnotatedCoinIndexSignature {
            signature,
            index: index as u64,
        })
        .collect();

        TestEcash {
            authorities,
            verification_keys,
            master_vk,
            coin_index_signatures,
        }
    }

    /// Fabricate a real threshold-signed ticketbook for `typ`. Distinct `seed`s
    /// yield distinct books (fresh user keypair per book).
    pub fn ticketbook(&self, typ: TicketType, seed: u64) -> IssuedTicketBook {
        let user = generate_keypair_user_from_seed(seed.to_be_bytes());
        let expiration_date = ecash_default_expiration_date();
        let expiration_ts = expiration_date.ecash_unix_timestamp();

        let (req, req_info) = withdrawal_request(user.secret_key(), expiration_ts, typ.encode())
            .expect("withdrawal request");

        let shares: Vec<PartialWallet> = self
            .authorities
            .iter()
            .zip(self.verification_keys.iter())
            .enumerate()
            .map(|(i, (kp, vk))| {
                let blinded = issue(
                    kp.secret_key(),
                    user.public_key(),
                    &req,
                    expiration_ts,
                    typ.encode(),
                )
                .expect("issue blind signature");
                issue_verify(vk, user.secret_key(), &blinded, &req_info, i as u64 + 1)
                    .expect("unblind share")
            })
            .collect();

        let wallet = aggregate_wallets(&self.master_vk, user.secret_key(), &shares, &req_info)
            .expect("aggregate wallets");

        IssuedTicketBook::new(
            wallet.into_wallet_signatures(),
            EPOCH_ID,
            user.secret_key().clone(),
            typ,
            expiration_date,
        )
    }

    /// The master verification key as the fetcher would serve it.
    pub fn epoch_verification_key(&self, epoch_id: u64) -> EpochVerificationKey {
        EpochVerificationKey {
            epoch_id,
            key: self.master_vk.clone(),
        }
    }

    /// The aggregated coin-index signatures as the fetcher would serve them.
    pub fn coin_index_signatures(&self, epoch_id: u64) -> AggregatedCoinIndicesSignatures {
        AggregatedCoinIndicesSignatures {
            epoch_id,
            signatures: self.coin_index_signatures.clone(),
        }
    }

    /// Number of signing authorities in the fixture set.
    pub fn num_authorities(&self) -> u64 {
        AUTHORITIES
    }

    /// The DKG threshold of the fixture set.
    pub fn threshold(&self) -> u64 {
        THRESHOLD
    }

    /// Authority `i`'s verification key share, bs58-encoded — the form a DKG
    /// contract `ContractVKShare` carries.
    pub fn vk_share_bs58(&self, i: usize) -> String {
        self.verification_keys[i].to_bs58()
    }

    /// Aggregated expiration-date signatures for the default expiration date,
    /// annotated the way the nym-api serves them. Values are parseable by the
    /// real client (which is all fetch-time consumers require — verification
    /// happens at spend time).
    pub fn expiration_date_signatures(&self) -> (Date, Vec<AnnotatedExpirationDateSignature>) {
        let expiration_date = ecash_default_expiration_date();
        let expiration_ts = expiration_date.ecash_unix_timestamp();
        let secret_keys: Vec<&SecretKeyAuth> =
            self.authorities.iter().map(|kp| kp.secret_key()).collect();
        let indices: Vec<u64> = (1..=AUTHORITIES).collect();
        let signatures = generate_expiration_date_signatures(
            expiration_ts,
            &secret_keys,
            &self.verification_keys,
            &self.master_vk,
            &indices,
        )
        .expect("expiration date signatures");
        let day: u32 = 24 * 60 * 60;
        let n = signatures.len() as u32;
        let annotated = signatures
            .into_iter()
            .enumerate()
            .map(|(i, signature)| AnnotatedExpirationDateSignature {
                signature,
                expiration_timestamp: expiration_ts,
                spending_timestamp: expiration_ts - (n - 1 - i as u32) * day,
            })
            .collect();
        (expiration_date, annotated)
    }
}

impl Default for TestEcash {
    fn default() -> Self {
        Self::new()
    }
}

/// A valid (random) gateway identity for spend attempts.
pub fn test_gateway_id() -> ed25519::PublicKey {
    let mut rng = rand::rngs::OsRng;
    *ed25519::KeyPair::new(&mut rng).public_key()
}
