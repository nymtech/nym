// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Spend-verifier regression tests. Every forgery is built by hand from an honestly issued wallet,
//! exactly as a malicious prover would on the wire.

use crate::common_types::Signature;
use crate::error::Result;
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::aggregation::{aggregate_verification_keys, aggregate_wallets};
use crate::scheme::coin_indices_signatures::CoinIndexSignature;
use crate::scheme::expiration_date_signatures::{find_index, ExpirationDateSignature};
use crate::scheme::keygen::{
    generate_keypair_user, ttp_keygen, SecretKeyAuth, VerificationKeyAuth,
};
use crate::scheme::withdrawal::{issue, issue_verify, withdrawal_request};
use crate::scheme::{compute_pay_info_hash, PartialWallet, PayInfo, Payment, WalletSignatures};
use crate::setup::Parameters;
use crate::tests::helpers::{
    generate_coin_indices_signatures, generate_expiration_date_signatures,
    payment_from_keys_and_expiration_date,
};
use crate::{date_scalar, ecash_group_parameters, type_scalar, EncodedDate, EncodedTicketType};
use itertools::izip;
use nym_bls12_381_fork::{G1Projective, G2Projective, Scalar};

const MIXNET_EXIT: EncodedTicketType = 1;
const WIREGUARD: EncodedTicketType = 2;
/// A coin index no authority has ever signed in these fixtures (the wallet holds index 0 only).
const UNSIGNED_INDEX: u64 = 1_000_000_000;
const SPEND_DATE: EncodedDate = 1701907200; // Dec 07 2023 00:00:00
const EXPIRATION_DATE: EncodedDate = 1702166400; // Dec 10 2023 00:00:00

/// Verification key shape under test.
#[derive(Clone, Copy, Debug)]
#[expect(clippy::enum_variant_names)]
enum KeyShape {
    /// Today's mainnet: DKG key_size 5, four betas.
    Mainnet4Beta,
    /// What ttp_keygen produces and what mainnet gets at key_size 6: five betas.
    Extended5Beta,
    /// Two components past anything the scheme reads; only an upper bound on key length can reject it.
    Oversized7Beta,
}

const KEY_SHAPES: [KeyShape; 3] = [
    KeyShape::Mainnet4Beta,
    KeyShape::Extended5Beta,
    KeyShape::Oversized7Beta,
];

/// An honestly issued one-coin wallet (index 0) plus everything its holder knows.
struct IssuedWallet {
    verification_key: VerificationKeyAuth,
    sk: Scalar,
    v: Scalar,
    exp: Scalar,
    wallet_signatures: WalletSignatures,
    dates_signatures: Vec<ExpirationDateSignature>,
    coin_indices_signatures: Vec<CoinIndexSignature>,
    pay_info: PayInfo,
    spend_date: EncodedDate,
    expiration_date: EncodedDate,
    t_type: EncodedTicketType,
}

/// Issue a one-coin wallet for `t_type` under a key of the requested shape.
fn issue_wallet(shape: KeyShape, t_type: EncodedTicketType) -> Result<IssuedWallet> {
    let params = Parameters::new(1);
    let user_keypair = generate_keypair_user();
    let indices: [u64; 3] = [1, 2, 3];

    // ttp_keygen yields gammas + 1 = 5 y-shares; the other shapes are the same key truncated or padded
    let authorities: Vec<SecretKeyAuth> = ttp_keygen(2, 3)?
        .iter()
        .map(|k| {
            let (x, mut ys) = k.secret_key().hazmat_to_raw();
            match shape {
                KeyShape::Mainnet4Beta => ys.truncate(4),
                KeyShape::Extended5Beta => {}
                KeyShape::Oversized7Beta => ys.extend(ecash_group_parameters().n_random_scalars(2)),
            }
            SecretKeyAuth::create_from_raw(x, ys)
        })
        .collect();
    let secret_keys: Vec<&SecretKeyAuth> = authorities.iter().collect();
    let verification_keys: Vec<VerificationKeyAuth> =
        authorities.iter().map(|k| k.verification_key()).collect();
    let verification_key = aggregate_verification_keys(&verification_keys, Some(&indices))?;

    let dates_signatures = generate_expiration_date_signatures(
        EXPIRATION_DATE,
        &secret_keys,
        &verification_keys,
        &verification_key,
        &indices,
    )?;
    let coin_indices_signatures = generate_coin_indices_signatures(
        &params,
        &secret_keys,
        &verification_keys,
        &verification_key,
        &indices,
    )?;

    let (req, req_info) = withdrawal_request(user_keypair.secret_key(), EXPIRATION_DATE, t_type)?;
    let blinded: Vec<_> = authorities
        .iter()
        .map(|sk| issue(sk, user_keypair.public_key(), &req, EXPIRATION_DATE, t_type))
        .collect::<Result<_>>()?;
    let shares: Vec<PartialWallet> = izip!(blinded.iter(), verification_keys.iter())
        .enumerate()
        .map(|(idx, (w, vk))| {
            issue_verify(vk, user_keypair.secret_key(), w, &req_info, idx as u64 + 1)
        })
        .collect::<Result<_>>()?;
    let wallet = aggregate_wallets(
        &verification_key,
        user_keypair.secret_key(),
        &shares,
        &req_info,
    )?;

    Ok(IssuedWallet {
        sk: user_keypair.secret_key().sk,
        v: *req_info.get_v(),
        exp: date_scalar(EXPIRATION_DATE),
        wallet_signatures: wallet.into_wallet_signatures(),
        verification_key,
        dates_signatures,
        coin_indices_signatures,
        pay_info: PayInfo {
            pay_info_bytes: [6u8; 72],
        },
        spend_date: SPEND_DATE,
        expiration_date: EXPIRATION_DATE,
        t_type,
    })
}

/// The randomised signatures and commitments every prover derives before choosing what to claim.
struct ShowMaterial {
    sig_prime: Signature,
    r: Scalar,
    kappa: G2Projective,
    sig_exp_prime: ExpirationDateSignature,
    r_e: Scalar,
    kappa_e: G2Projective,
    o_c: Scalar,
    cc: G1Projective,
}

fn honest_show(fx: &IssuedWallet) -> ShowMaterial {
    let grp = ecash_group_parameters();
    let gamma1 = grp.gamma_idx(1).unwrap();
    let vk = &fx.verification_key;

    let (sig_prime, r) = fx.wallet_signatures.signature().blind_and_randomise();
    let kappa = grp.gen2() * r
        + vk.alpha
        + vk.beta_g2[0] * fx.sk
        + vk.beta_g2[1] * fx.v
        + vk.beta_g2[2] * fx.exp;

    let date_idx = find_index(fx.spend_date, fx.expiration_date).unwrap();
    let (sig_exp_prime, r_e) = fx.dates_signatures[date_idx].blind_and_randomise();
    let kappa_e = grp.gen2() * r_e + vk.alpha + vk.beta_g2[0] * fx.exp;

    let o_c = grp.random_scalar();
    let cc = grp.gen1() * o_c + gamma1 * fx.v;

    ShowMaterial {
        sig_prime,
        r,
        kappa,
        sig_exp_prime,
        r_e,
        kappa_e,
        o_c,
        cc,
    }
}

/// Serial, tag and commitment openings for one coin at `index`, paid at position `k`.
struct CoinMaterial {
    lk: Scalar,
    o_a: Scalar,
    aa: G1Projective,
    mu: Scalar,
    o_mu: Scalar,
    ss: G1Projective,
    tt: G1Projective,
    rr: Scalar,
}

fn honest_coin(fx: &IssuedWallet, show: &ShowMaterial, index: u64, k: u64) -> CoinMaterial {
    let grp = ecash_group_parameters();
    let gamma1 = grp.gamma_idx(1).unwrap();

    let lk = Scalar::from(index);
    let o_a = grp.random_scalar();
    let aa = grp.gen1() * o_a + gamma1 * lk;
    let mu: Scalar = Option::from((fx.v + lk + Scalar::from(1u64)).invert()).unwrap();
    let o_mu = -((o_a + show.o_c) * mu);
    let rr = compute_pay_info_hash(&fx.pay_info, k);
    let ss = grp.delta() * mu;
    let tt = grp.gen1() * fx.sk + (grp.gen1() * mu) * rr;

    CoinMaterial {
        lk,
        o_a,
        aa,
        mu,
        o_mu,
        ss,
        tt,
        rr,
    }
}

/// A coin-index signature show: the omega sent on the wire and the kappa_k the prover claims it opens.
struct CoinIndexShow {
    omega: CoinIndexSignature,
    blinding: Scalar,
    kappa_k: G2Projective,
}

/// Randomise the wallet's only real signature (index 0) but claim `claimed_index` in kappa_k.
fn show_index0_signature_as(fx: &IssuedWallet, claimed_index: u64) -> CoinIndexShow {
    let grp = ecash_group_parameters();
    let vk = &fx.verification_key;
    let (omega, blinding) = fx.coin_indices_signatures[0].blind_and_randomise();
    let kappa_k = grp.gen2() * blinding + vk.alpha + vk.beta_g2[0] * Scalar::from(claimed_index);
    CoinIndexShow {
        omega,
        blinding,
        kappa_k,
    }
}

/// The witness for `coins`; `r_k` is supplied by the caller (empty reproduces provers from before the fix).
fn honest_witness<'a>(
    show: &ShowMaterial,
    attributes: &'a [&'a Scalar],
    coins: &[CoinMaterial],
    r_k: Vec<Scalar>,
) -> SpendWitness<'a> {
    SpendWitness {
        attributes,
        r: show.r,
        o_c: show.o_c,
        lk: coins.iter().map(|c| c.lk).collect(),
        o_a: coins.iter().map(|c| c.o_a).collect(),
        mu: coins.iter().map(|c| c.mu).collect(),
        o_mu: coins.iter().map(|c| c.o_mu).collect(),
        r_k,
        r_e: show.r_e,
    }
}

/// Construct the proof over the chosen statement and wrap everything into a wire `Payment`.
fn assemble(
    fx: &IssuedWallet,
    show: &ShowMaterial,
    coins: &[CoinMaterial],
    index_shows: &[CoinIndexShow],
    witness: &SpendWitness,
    t_type_shown: EncodedTicketType,
) -> Payment {
    let spend_value = coins.len() as u64;
    let rr: Vec<Scalar> = coins.iter().map(|c| c.rr).collect();
    let instance = SpendInstance {
        kappa: show.kappa,
        cc: show.cc,
        aa: coins.iter().map(|c| c.aa).collect(),
        ss: coins.iter().map(|c| c.ss).collect(),
        tt: coins.iter().map(|c| c.tt).collect(),
        kappa_k: index_shows.iter().map(|s| s.kappa_k).collect(),
        kappa_e: show.kappa_e,
    };
    let zk_proof = SpendProof::construct(
        &instance,
        witness,
        &fx.verification_key,
        &rr,
        &fx.pay_info,
        spend_value,
    )
    .unwrap();

    Payment {
        kappa: show.kappa,
        kappa_e: show.kappa_e,
        sig: show.sig_prime,
        sig_exp: show.sig_exp_prime,
        kappa_k: instance.kappa_k,
        omega: index_shows.iter().map(|s| s.omega).collect(),
        ss: instance.ss,
        tt: instance.tt,
        aa: instance.aa,
        spend_value,
        cc: show.cc,
        t_type: t_type_shown,
        zk_proof,
    }
}

fn verify(fx: &IssuedWallet, payment: &Payment) -> Result<()> {
    payment.spend_verify(&fx.verification_key, &fx.pay_info, fx.spend_date)
}

/// Whether the prover proves the blinding factors behind kappa_k (absent from every pre-fix proof).
#[derive(Clone, Copy)]
enum CoinIndexRelation {
    Omitted,
    Proven,
}

/// Serial at `serial_index` while the coin-index show reuses the real index-0 signature and claims `claimed_index`.
fn forge_unbound_coin_index(
    fx: &IssuedWallet,
    serial_index: u64,
    claimed_index: u64,
    relation: CoinIndexRelation,
) -> Payment {
    let show = honest_show(fx);
    let coins = [honest_coin(fx, &show, serial_index, 0)];
    let index_shows = [show_index0_signature_as(fx, claimed_index)];
    let attributes = [&fx.sk, &fx.v, &fx.exp];
    let r_k = match relation {
        CoinIndexRelation::Omitted => vec![],
        CoinIndexRelation::Proven => vec![index_shows[0].blinding],
    };
    let witness = honest_witness(&show, &attributes, &coins, r_k);
    assemble(fx, &show, &coins, &index_shows, &witness, fx.t_type)
}

/// Honest index-0 show with attacker-chosen serial and tag; `omit_mu` drops the mu / o_mu witnesses so tt_ss, tt_tt and tt_gamma1 fall out of the challenge.
fn forge_arbitrary_serial_and_tag(fx: &IssuedWallet, serial_seed: u64, omit_mu: bool) -> Payment {
    let grp = ecash_group_parameters();
    let show = honest_show(fx);
    let mut coin = honest_coin(fx, &show, 0, 0);
    coin.ss = grp.gen1() * Scalar::from(serial_seed);
    coin.tt = grp.gen1() * Scalar::from(serial_seed.wrapping_add(0xDEAD));
    let coins = [coin];
    let index_shows = [show_index0_signature_as(fx, 0)];
    let attributes = [&fx.sk, &fx.v, &fx.exp];
    let mut witness = honest_witness(&show, &attributes, &coins, vec![index_shows[0].blinding]);
    if omit_mu {
        witness.mu.clear();
        witness.o_mu.clear();
    }
    assemble(fx, &show, &coins, &index_shows, &witness, fx.t_type)
}

/// Display `t_type_shown` for a wallet signed under fx.t_type; with `hidden_type_delta` a fourth hidden attribute cancels the public type inside kappa.
fn forge_hidden_ticket_type_swap(
    fx: &IssuedWallet,
    t_type_shown: EncodedTicketType,
    hidden_type_delta: bool,
) -> Payment {
    let vk = &fx.verification_key;
    let mut show = honest_show(fx);
    let w = type_scalar(fx.t_type) - type_scalar(t_type_shown);
    if hidden_type_delta {
        show.kappa += vk.beta_g2[3] * w;
    }
    let coins = [honest_coin(fx, &show, 0, 0)];
    let index_shows = [show_index0_signature_as(fx, 0)];
    let attrs4 = [&fx.sk, &fx.v, &fx.exp, &w];
    let attributes: &[&Scalar] = if hidden_type_delta {
        &attrs4
    } else {
        &attrs4[..3]
    };
    let witness = honest_witness(&show, attributes, &coins, vec![index_shows[0].blinding]);
    assemble(fx, &show, &coins, &index_shows, &witness, t_type_shown)
}

/// Honest coin at an index nobody signed; `identity_omega` sends omega = (O, O), otherwise the real index-0 signature (wrong index, the control).
fn forge_unsigned_index_omega(
    fx: &IssuedWallet,
    unsigned_index: u64,
    identity_omega: bool,
) -> Payment {
    let grp = ecash_group_parameters();
    let vk = &fx.verification_key;
    let show = honest_show(fx);
    let coins = [honest_coin(fx, &show, unsigned_index, 0)];
    let index_show = if identity_omega {
        let blinding = grp.random_scalar();
        let inf = G1Projective::identity();
        CoinIndexShow {
            omega: Signature { h: inf, s: inf },
            blinding,
            kappa_k: grp.gen2() * blinding
                + vk.alpha
                + vk.beta_g2[0] * Scalar::from(unsigned_index),
        }
    } else {
        show_index0_signature_as(fx, unsigned_index)
    };
    let index_shows = [index_show];
    let attributes = [&fx.sk, &fx.v, &fx.exp];
    let witness = honest_witness(&show, &attributes, &coins, vec![index_shows[0].blinding]);
    assemble(fx, &show, &coins, &index_shows, &witness, fx.t_type)
}

/// Two coins at unsigned indices anchored on the real index-0 signature with weights c0*l0 + c1*l1 = 0, so the per-term pairing errors cancel in a coefficient-free batch.
fn forge_cancelling_omega_pair(fx: &IssuedWallet, idx0: u64, idx1: u64) -> Payment {
    let grp = ecash_group_parameters();
    let vk = &fx.verification_key;
    let show = honest_show(fx);
    let coins = [
        honest_coin(fx, &show, idx0, 0),
        honest_coin(fx, &show, idx1, 1),
    ];
    let Signature { h: h0, s: s0 } = fx.coin_indices_signatures[0];
    let weights = [Scalar::from(idx1), -Scalar::from(idx0)];
    let index_shows: Vec<CoinIndexShow> = coins
        .iter()
        .zip(weights)
        .map(|(coin, c)| {
            let blinding = grp.random_scalar();
            CoinIndexShow {
                omega: Signature {
                    h: h0 * c,
                    s: (s0 + h0 * blinding) * c,
                },
                blinding,
                kappa_k: grp.gen2() * blinding + vk.alpha + vk.beta_g2[0] * coin.lk,
            }
        })
        .collect();
    let attributes = [&fx.sk, &fx.v, &fx.exp];
    let r_k = index_shows.iter().map(|s| s.blinding).collect();
    let witness = honest_witness(&show, &attributes, &coins, r_k);
    assemble(fx, &show, &coins, &index_shows, &witness, fx.t_type)
}

#[test]
fn fixture_key_shapes_match_their_names() -> Result<()> {
    assert_eq!(
        issue_wallet(KeyShape::Mainnet4Beta, MIXNET_EXIT)?
            .verification_key
            .beta_g2
            .len(),
        4
    );
    assert_eq!(
        issue_wallet(KeyShape::Extended5Beta, MIXNET_EXIT)?
            .verification_key
            .beta_g2
            .len(),
        5
    );
    assert_eq!(
        issue_wallet(KeyShape::Oversized7Beta, MIXNET_EXIT)?
            .verification_key
            .beta_g2
            .len(),
        7
    );
    Ok(())
}

/// Provers from before the coin-index binding send no relation for it; the verifier must keep accepting them until every prover does.
#[test]
fn honest_legacy_payment_verifies_in_all_key_shapes() -> Result<()> {
    for shape in KEY_SHAPES {
        let fx = issue_wallet(shape, MIXNET_EXIT)?;
        let legacy = forge_unbound_coin_index(&fx, 0, 0, CoinIndexRelation::Omitted);
        assert!(verify(&fx, &legacy).is_ok(), "{shape:?}");
    }
    Ok(())
}

/// The fixed prover's shape is accepted by the current verifier, so clients can ship it before gateways update.
#[test]
fn honest_payment_with_coin_index_relation_verifies_in_all_key_shapes() -> Result<()> {
    for shape in KEY_SHAPES {
        let fx = issue_wallet(shape, MIXNET_EXIT)?;
        let fixed = forge_unbound_coin_index(&fx, 0, 0, CoinIndexRelation::Proven);
        assert!(verify(&fx, &fixed).is_ok(), "{shape:?}");
    }
    Ok(())
}

/// A hidden fourth attribute that cancels the public ticket type must fail under every key shape, not only under today's 4-beta keys.
#[test]
fn hidden_fourth_attribute_is_rejected_in_all_key_shapes() -> Result<()> {
    for shape in KEY_SHAPES {
        let fx = issue_wallet(shape, MIXNET_EXIT)?;
        // honest display of the real type keeps working
        assert!(
            verify(&fx, &forge_hidden_ticket_type_swap(&fx, MIXNET_EXIT, false)).is_ok(),
            "{shape:?}"
        );
        // the swap with the cancelling hidden attribute
        let forged = forge_hidden_ticket_type_swap(&fx, WIREGUARD, true);
        assert_eq!(forged.t_type, WIREGUARD);
        assert!(verify(&fx, &forged).is_err(), "{shape:?}");
        // the swap without it is bound by check_signature_validity and fails too
        assert!(
            verify(&fx, &forge_hidden_ticket_type_swap(&fx, WIREGUARD, false)).is_err(),
            "{shape:?}"
        );
    }
    Ok(())
}

/// The production prover must send the blinding factors behind kappa_k so the verifier can bind each signed index to its serial.
#[test]
fn wallet_spend_proves_coin_index_binding() -> Result<()> {
    let keypairs = ttp_keygen(2, 3)?;
    let (payment, _) =
        payment_from_keys_and_expiration_date(&keypairs, &[1, 2, 3], EXPIRATION_DATE, MIXNET_EXIT)?;
    assert!(payment.proves_coin_index_binding());
    Ok(())
}

/// With the relation present, a kappa_k claiming a different index than the serial's is rejected.
#[test]
fn claimed_index_is_bound_once_the_relation_is_proven() -> Result<()> {
    let fx = issue_wallet(KeyShape::Mainnet4Beta, MIXNET_EXIT)?;
    let bound = forge_unbound_coin_index(&fx, UNSIGNED_INDEX, 0, CoinIndexRelation::Proven);
    assert!(bound.proves_coin_index_binding());
    assert!(verify(&fx, &bound).is_err());
    Ok(())
}

/// Flips to is_err() once responses_r_k is pinned. The relation is absent from every pre-fix proof,
/// so until then this forgery is indistinguishable from an honest legacy payment.
#[test]
fn unbound_coin_index_is_still_accepted_without_the_relation() -> Result<()> {
    let fx = issue_wallet(KeyShape::Mainnet4Beta, MIXNET_EXIT)?;
    let forged = forge_unbound_coin_index(&fx, UNSIGNED_INDEX, 0, CoinIndexRelation::Omitted);
    assert!(!forged.proves_coin_index_binding());
    assert!(verify(&fx, &forged).is_ok());
    Ok(())
}

/// Two individually invalid omegas whose pairing errors cancel pass a coefficient-free batch; each coin must be checked on its own.
#[test]
fn cancelling_omega_pair_is_rejected() -> Result<()> {
    let fx = issue_wallet(KeyShape::Mainnet4Beta, MIXNET_EXIT)?;
    let forged = forge_cancelling_omega_pair(&fx, UNSIGNED_INDEX, 2 * UNSIGNED_INDEX);
    // this construction has nothing to do with identities
    assert!(!bool::from(forged.omega[0].is_at_infinity()));
    assert!(!bool::from(forged.omega[1].is_at_infinity()));
    assert_ne!(forged.ss[0], forged.ss[1]);
    assert!(verify(&fx, &forged).is_err());
    Ok(())
}

/// An omega at infinity would make its pairing term vanish; both the payment-level check and the primitive must refuse it.
#[test]
fn identity_omega_is_rejected() -> Result<()> {
    let fx = issue_wallet(KeyShape::Mainnet4Beta, MIXNET_EXIT)?;
    let forged = forge_unsigned_index_omega(&fx, UNSIGNED_INDEX, true);
    assert!(bool::from(forged.omega[0].is_at_infinity()));
    assert!(verify(&fx, &forged).is_err());
    // the real signature shown for the wrong index is rejected as well
    assert!(verify(&fx, &forge_unsigned_index_omega(&fx, UNSIGNED_INDEX, false)).is_err());
    Ok(())
}

/// Dropping the mu / o_mu responses truncates tt_ss, tt_tt and tt_gamma1 out of the challenge and leaves the serial and tag unconstrained.
#[test]
fn omitted_mu_responses_are_rejected() -> Result<()> {
    let fx = issue_wallet(KeyShape::Mainnet4Beta, MIXNET_EXIT)?;
    let forged = forge_arbitrary_serial_and_tag(&fx, UNSIGNED_INDEX, true);
    // the serial the verifier would accept is a bare attacker-chosen point, not delta / (v + l + 1)
    assert_eq!(
        forged.ss[0],
        ecash_group_parameters().gen1() * Scalar::from(UNSIGNED_INDEX)
    );
    assert!(verify(&fx, &forged).is_err());
    // with the mu responses present, tt_ss binds the serial and the same forgery fails for the right reason
    assert!(verify(
        &fx,
        &forge_arbitrary_serial_and_tag(&fx, UNSIGNED_INDEX, false)
    )
    .is_err());
    Ok(())
}
