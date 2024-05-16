// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::cell::Cell;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;
use rand::Rng;
use time::OffsetDateTime;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::constants;
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::expiration_date_signatures::{find_index, ExpirationDateSignature};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{CoinIndexSignature, GroupParameters, Parameters};
use crate::traits::Bytable;
use crate::utils::{
    check_bilinear_pairing, hash_to_scalar, try_deserialize_g1_projective,
    try_deserialize_g2_projective, Signature, SignerIndex,
};
use crate::{Attribute, Base58};

pub mod aggregation;
pub mod expiration_date_signatures;
pub mod identify;
pub mod keygen;
pub mod setup;
pub mod withdrawal;

/// The struct represents a partial wallet with essential components for a payment transaction.
///
/// A `PartialWallet` includes a Pointcheval-Sanders signature (`sig`),
/// a scalar value (`v`) representing the wallet's secret, an optional
/// `SignerIndex` (`idx`) indicating the signer's index, and an expiration date (`expiration_date`).
///
#[derive(Debug, Clone, PartialEq, Zeroize, ZeroizeOnDrop)]
pub struct PartialWallet {
    #[zeroize(skip)]
    sig: Signature,
    v: Scalar,
    idx: SignerIndex,
    expiration_date: Scalar,
}

impl PartialWallet {
    pub fn signature(&self) -> &Signature {
        &self.sig
    }

    pub fn index(&self) -> SignerIndex {
        self.idx
    }
    pub fn expiration_date(&self) -> Scalar {
        self.expiration_date
    }

    /// Converts the `PartialWallet` to a fixed-size byte array.
    ///
    /// The resulting byte array has a length of 168 bytes and contains serialized
    /// representations of the `Signature` (`sig`), scalar value (`v`),
    /// expiration date (`expiration_date`), and `idx` fields of the `PartialWallet` struct.
    ///
    /// # Returns
    ///
    /// A fixed-size byte array (`[u8; 168]`) representing the serialized form of the `PartialWallet`.
    ///
    pub fn to_bytes(&self) -> [u8; 168] {
        let mut bytes = [0u8; 168];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..160].copy_from_slice(&self.expiration_date.to_bytes());
        bytes[160..168].copy_from_slice(&self.idx.to_le_bytes());
        bytes
    }
}

impl TryFrom<&[u8]> for PartialWallet {
    type Error = CompactEcashError;

    /// Convert a byte slice into a `PartialWallet` instance.
    ///
    /// This function performs deserialization on the provided byte slice, which
    /// represent a serialized `PartialWallet`.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A reference to the byte slice to be deserialized.
    ///
    /// # Returns
    ///
    /// A `Result` containing the deserialized `PartialWallet` if successful, or a
    /// `CompactEcashError` indicating the reason for failure.
    fn try_from(bytes: &[u8]) -> Result<PartialWallet> {
        const SIGNATURE_BYTES: usize = 96;
        const V_BYTES: usize = 32;
        const EXPIRATION_DATE_BYTES: usize = 32;
        const IDX_BYTES: usize = 8;
        const EXPECTED_LENGTH: usize =
            SIGNATURE_BYTES + V_BYTES + EXPIRATION_DATE_BYTES + IDX_BYTES;

        if bytes.len() != EXPECTED_LENGTH {
            return Err(CompactEcashError::Deserialization(format!(
                "PartialWallet should be exactly {} bytes, got {}",
                EXPECTED_LENGTH,
                bytes.len()
            )));
        }

        let sig_bytes: &[u8; SIGNATURE_BYTES] = bytes
            .get(..SIGNATURE_BYTES)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                CompactEcashError::Deserialization("Failed to convert Signature bytes".to_string())
            })?;

        let v_bytes: &[u8; V_BYTES] = bytes
            .get(SIGNATURE_BYTES..(SIGNATURE_BYTES + V_BYTES))
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                CompactEcashError::Deserialization("Failed to convert Scalar bytes".to_string())
            })?;

        let expiration_date_bytes: &[u8; EXPIRATION_DATE_BYTES] = bytes
            .get((SIGNATURE_BYTES + V_BYTES)..(SIGNATURE_BYTES + V_BYTES + EXPIRATION_DATE_BYTES))
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                CompactEcashError::Deserialization(
                    "Failed to convert Expiration Date bytes".to_string(),
                )
            })?;

        let idx_bytes: &[u8; IDX_BYTES] = bytes
            .get((SIGNATURE_BYTES + V_BYTES + EXPIRATION_DATE_BYTES)..)
            .and_then(|slice| slice.try_into().ok())
            .ok_or_else(|| {
                CompactEcashError::Deserialization(
                    "Failed to convert SignerIndex bytes".to_string(),
                )
            })?;

        let sig = Signature::try_from(sig_bytes.as_slice())?;
        let maybe_v = Scalar::from_bytes(v_bytes);
        let v = if maybe_v.is_some().into() {
            //SAFETY: here we know maybe_v is Some()
            maybe_v.unwrap()
        } else {
            return Err(CompactEcashError::Deserialization(
                "Failed to convert wallet secret bytes".to_string(),
            ));
        };
        let expiration_date = Scalar::from_bytes(expiration_date_bytes).unwrap();
        let idx = u64::from_le_bytes(*idx_bytes);

        Ok(PartialWallet {
            sig,
            v,
            idx,
            expiration_date,
        })
    }
}

/// The struct represents a wallet with essential components for a payment transaction.
///
/// A `Wallet` includes a Pointcheval-Sanders signature (`sig`),
/// a scalar value (`v`) representing the wallet's secret, an optional
/// `SignerIndex` (`idx`) indicating the signer's index, and an expiration date (`expiration_date`)
/// and the a u64 value ('l') indicating the remaining number of coins in the wallet.
///
#[derive(Debug, Clone, PartialEq, Zeroize, ZeroizeOnDrop)]
pub struct Wallet {
    #[zeroize(skip)]
    sig: Signature,
    v: Scalar,
    expiration_date: Scalar,
    #[zeroize(skip)]
    pub l: Cell<u64>,
}

/// Computes the hash of payment information concatenated with a numeric value.
///
/// This function takes a `PayInfo` structure and a numeric value `k`, and
/// concatenates the serialized `payinfo` field of `PayInfo` with the little-endian
/// byte representation of `k`. The resulting byte sequence is then hashed to produce
/// a scalar value using the `hash_to_scalar` function.
///
/// # Arguments
///
/// * `pay_info` - A reference to the `PayInfo` structure containing payment information.
/// * `k` - A numeric value used in the hash computation.
///
/// # Returns
///
/// A `Scalar` value representing the hash of the concatenated byte sequence.
///
pub fn compute_pay_info_hash(pay_info: &PayInfo, k: u64) -> Scalar {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&pay_info.pay_info_bytes);
    bytes.extend_from_slice(&k.to_le_bytes());
    hash_to_scalar(bytes)
}

impl Wallet {
    pub fn signature(&self) -> &Signature {
        &self.sig
    }

    pub fn v(&self) -> Scalar {
        self.v
    }

    pub fn l(&self) -> u64 {
        self.l.get()
    }

    pub fn expiration_date(&self) -> Scalar {
        self.expiration_date
    }

    /// Converts the `Wallet` to a fixed-size byte array.
    ///
    /// The resulting byte array has a length of 168 bytes and contains serialized
    /// representations of the `Signature` (`sig`), scalar value (`v`), and
    /// expiration date (`expiration_date`) fields of the `PartialWallet` struct.
    /// If the `idx` field is present, its bytes are also included in the byte array.
    ///
    /// # Returns
    ///
    /// A fixed-size byte array (`[u8; 168]`) representing the serialized form of the `Wallet`.
    ///
    pub fn to_bytes(&self) -> [u8; 168] {
        let mut bytes = [0u8; 168];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..160].copy_from_slice(&self.expiration_date.to_bytes());
        bytes[160..168].copy_from_slice(&self.l.get().to_le_bytes());
        bytes
    }

    fn check_remaining_allowance(&self, params: &Parameters, spend_value: u64) -> Result<()> {
        if self.l() + spend_value > params.get_total_coins() {
            Err(CompactEcashError::Spend(
                "The amount you want to spend exceeds remaining wallet allowance ".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Performs a spending operation with the given parameters, updating the wallet and generating a payment.
    ///
    /// # Arguments
    ///
    /// * `params` - The system parameters.
    /// * `verification_key` - The global verification key.
    /// * `sk_user` - The secret key of the user who wants to spend from their wallet.
    /// * `pay_info` - Unique information related to the payment.
    /// * `bench_flag` - A flag indicating whether to perform benchmarking.
    /// * `spend_value` - The amount to spend from the wallet.
    /// * `valid_dates_signatures` - A list of signatures on valid dates during which we can spend from the wallet.
    /// * `coin_indices_signatures` - A list of signatures on coin indices.
    /// * `spend_date` - The date on which the spending occurs.
    ///
    /// # Returns
    ///
    /// A tuple containing the generated payment and a reference to the updated wallet, or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn spend(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        bench_flag: bool,
        spend_value: u64,
        valid_dates_signatures: Vec<ExpirationDateSignature>,
        coin_indices_signatures: Vec<CoinIndexSignature>,
        spend_date: Scalar,
    ) -> Result<(Payment, &Self)> {
        // Extract group parameters
        let grp_params = params.grp();

        // Wallet attributes needed for spending
        let attributes = vec![sk_user.sk, self.v(), self.expiration_date()];

        // Check if there is enough remaining allowance in the wallet
        self.check_remaining_allowance(params, spend_value)?;

        // Randomize wallet signature
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grp_params);

        // compute kappa (i.e., blinded attributes for show) to prove possession of the wallet signature
        let kappa = compute_kappa(
            grp_params,
            verification_key,
            &attributes,
            sign_blinding_factor,
        );

        // Randomise the expiration date signature for the date when we want to perform the spending, and compute kappa_e to prove possession of
        // the expiration signature
        let date_signature_index = find_index(spend_date, self.expiration_date)?;
        //SAFETY : find_index eiter returns a valid index or an error. The unwrap is therefore fine
        #[allow(clippy::unwrap_used)]
        let date_signature: ExpirationDateSignature = valid_dates_signatures
            .get(date_signature_index)
            .unwrap()
            .clone();
        let (date_signature_prime, date_sign_blinding_factor) =
            date_signature.randomise(grp_params);
        // compute kappa_e to prove possession of the expiration signature
        let kappa_e: G2Projective = grp_params.gen2() * date_sign_blinding_factor
            + verification_key.alpha
            + verification_key.beta_g2.first().unwrap() * self.expiration_date();

        // pick random openings o_c and compute commitments C to v (wallet secret)
        let o_c = grp_params.random_scalar();
        let cc = grp_params.gen1() * o_c + grp_params.gamma_idx(0).unwrap() * self.v();

        let mut aa: Vec<G1Projective> = Default::default();
        let mut ss: Vec<G1Projective> = Default::default();
        let mut tt: Vec<G1Projective> = Default::default();
        let mut rr: Vec<Scalar> = Default::default();
        let mut o_a: Vec<Scalar> = Default::default();
        let mut o_mu: Vec<Scalar> = Default::default();
        let mut mu: Vec<Scalar> = Default::default();
        let r_k_vec: Vec<Scalar> = Default::default();
        let mut kappa_k_vec: Vec<G2Projective> = Default::default();
        let mut lk_vec: Vec<Scalar> = Default::default();

        let mut coin_indices_signatures_prime: Vec<CoinIndexSignature> = Default::default();
        for k in 0..spend_value {
            let lk = self.l() + k;
            lk_vec.push(Scalar::from(lk));

            // compute hashes R_k = H(payinfo, k)
            let rr_k = compute_pay_info_hash(pay_info, k);
            rr.push(rr_k);

            let o_a_k = grp_params.random_scalar();
            o_a.push(o_a_k);
            let aa_k =
                grp_params.gen1() * o_a_k + grp_params.gamma_idx(0).unwrap() * Scalar::from(lk);
            aa.push(aa_k);

            // compute the serial numbers
            let ss_k = pseudorandom_f_delta_v(grp_params, self.v(), lk);
            ss.push(ss_k);
            // compute the identification tags
            let tt_k = grp_params.gen1() * sk_user.sk
                + pseudorandom_f_g_v(grp_params, self.v(), lk) * rr_k;
            tt.push(tt_k);

            // compute values mu, o_mu, lambda, o_lambda
            let mu_k: Scalar = (self.v() + Scalar::from(lk) + Scalar::from(1))
                .invert()
                .unwrap();
            mu.push(mu_k);

            let o_mu_k = ((o_a_k + o_c) * mu_k).neg();
            o_mu.push(o_mu_k);

            // Randomize the coin index signatures and compute kappa_k to prove possession of each coin's signature
            // This involves iterating over the signatures corresponding to the coins we want to spend in this payment.
            //SAFETY : Earlier `check_remaining_allowance` ensures we don't do out of of bound here
            #[allow(clippy::unwrap_used)]
            let coin_sign: CoinIndexSignature = *coin_indices_signatures.get(lk as usize).unwrap();
            let (coin_sign_prime, coin_sign_blinding_factor) = coin_sign.randomise(grp_params);
            coin_indices_signatures_prime.push(coin_sign_prime);
            let kappa_k: G2Projective = grp_params.gen2() * coin_sign_blinding_factor
                + verification_key.alpha
                + verification_key.beta_g2.first().unwrap() * Scalar::from(lk);
            kappa_k_vec.push(kappa_k);
        }

        // construct the zkp proof
        let spend_instance = SpendInstance {
            kappa,
            cc,
            aa: aa.clone(),
            ss: ss.clone(),
            tt: tt.clone(),
            kappa_k: kappa_k_vec.clone(),
            kappa_e,
        };
        let spend_witness = SpendWitness {
            attributes,
            r: sign_blinding_factor,
            o_c,
            lk: lk_vec,
            o_a,
            mu,
            o_mu,
            r_k: r_k_vec,
            r_e: date_sign_blinding_factor,
        };

        let zk_proof = SpendProof::construct(
            params,
            &spend_instance,
            &spend_witness,
            verification_key,
            &rr,
            pay_info,
            spend_value,
        );

        // output pay and updated wallet
        let pay = Payment {
            kappa,
            kappa_e,
            sig: signature_prime,
            sig_exp: date_signature_prime,
            kappa_k: kappa_k_vec.clone(),
            omega: coin_indices_signatures_prime,
            ss: ss.clone(),
            tt: tt.clone(),
            aa: aa.clone(),
            spend_value,
            cc,
            zk_proof,
        };

        // The number of samples collected by the benchmark process is way higher than the
        // MAX_WALLET_VALUE we ever consider. Thus, we would execute the spending too many times
        // and the initial condition at the top of this function will crush. Thus, we need a
        // benchmark flag to signal that we don't want to increase the spending couter but only
        // care about the function performance.
        if !bench_flag {
            let current_l = self.l();
            self.l.set(current_l + spend_value);
        }

        Ok((pay, self))
    }
}

impl TryFrom<&[u8]> for Wallet {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Wallet> {
        if bytes.len() != 168 {
            return Err(CompactEcashError::Deserialization(format!(
                "Wallet should be exactly 168 bytes, got {}",
                bytes.len()
            )));
        }
        //SAFETY : slice to array conversions after a length check
        #[allow(clippy::unwrap_used)]
        let sig_bytes: &[u8; 96] = &bytes[..96].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let v_bytes: &[u8; 32] = &bytes[96..128].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let expiration_date_bytes: &[u8; 32] = &bytes[128..160].try_into().unwrap();
        #[allow(clippy::unwrap_used)]
        let l_bytes: &[u8; 8] = &bytes[160..168].try_into().unwrap();

        let sig = Signature::try_from(sig_bytes.as_slice())?;
        let v = Scalar::from_bytes(v_bytes).unwrap();
        let expiration_date = Scalar::from_bytes(expiration_date_bytes).unwrap();
        let l = Cell::new(u64::from_le_bytes(*l_bytes));

        Ok(Wallet {
            sig,
            v,
            expiration_date,
            l,
        })
    }
}

impl Bytable for Wallet {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> std::result::Result<Self, CompactEcashError> {
        Wallet::try_from(slice)
    }
}

impl Base58 for Wallet {}

pub fn pseudorandom_f_delta_v(params: &GroupParameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
    params.delta() * pow
}

pub fn pseudorandom_f_g_v(params: &GroupParameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
    params.gen1() * pow
}

/// Computes the value of kappa (blinded private attributes for show) for proving possession of the wallet signature.
///
/// This function calculates the value of kappa, which is used to prove possession of the wallet signature in the zero-knowledge proof.
///
/// # Arguments
///
/// * `params` - A reference to the group parameters required for the computation.
/// * `verification_key` - The global verification key of the signing authorities.
/// * `attributes` - A slice of private attributes associated with the wallet.
/// * `blinding_factor` - The blinding factor used used to randomise the wallet's signature.
///
/// # Returns
///
/// A `G2Projective` element representing the computed value of kappa.
///
pub fn compute_kappa(
    params: &GroupParameters,
    verification_key: &VerificationKeyAuth,
    attributes: &[Attribute],
    blinding_factor: Scalar,
) -> G2Projective {
    params.gen2() * blinding_factor
        + verification_key.alpha
        + attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(priv_attr, beta_i)| beta_i * priv_attr)
            .sum::<G2Projective>()
}

/// Represents the unique payment information associated with the payment.
/// The bytes representing the payment information encode the public key of the
/// provider with whom you are spending the payment, timestamp and a unique random 32 bytes.
///
/// # Fields
///
/// * `payinfo_bytes` - An array of bytes representing the payment information.
///
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PayInfo {
    pub pay_info_bytes: [u8; 72],
}

impl PayInfo {
    /// Generates a new `PayInfo` instance with random bytes, a timestamp, and a provider public key.
    ///
    /// # Arguments
    ///
    /// * `provider_pk` - The public key of the payment provider.
    ///
    /// # Returns
    ///
    /// A new `PayInfo` instance.
    ///
    pub fn generate_pay_info(provider_pk: [u8; 32]) -> PayInfo {
        let mut pay_info_bytes = [0u8; 72];

        // Generating random bytes using the `rand` crate
        rand::thread_rng().fill(&mut pay_info_bytes[..32]);

        // Adding timestamp bytes
        let timestamp = OffsetDateTime::now_utc().unix_timestamp();
        pay_info_bytes[32..40].copy_from_slice(&timestamp.to_be_bytes());

        // Adding provider public key bytes
        pay_info_bytes[40..].copy_from_slice(&provider_pk);

        PayInfo { pay_info_bytes }
    }

    pub fn timestamp(&self) -> i64 {
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        i64::from_be_bytes(self.pay_info_bytes[32..40].try_into().unwrap())
    }

    pub fn pk(&self) -> [u8; 32] {
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        self.pay_info_bytes[40..].try_into().unwrap()
    }
}

impl Bytable for PayInfo {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.pay_info_bytes.to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> std::result::Result<Self, CompactEcashError> {
        if slice.len() != 72 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for PayInfo deserialization".to_string(),
            ));
        }
        //safety : we checked that slices length is exactly 72, hence this unwrap won't fail
        #[allow(clippy::unwrap_used)]
        Ok(Self {
            pay_info_bytes: slice.try_into().unwrap(),
        })
    }
}

impl Base58 for PayInfo {}

#[derive(Debug, Clone, PartialEq)]
pub struct Payment {
    pub kappa: G2Projective,
    pub kappa_e: G2Projective,
    pub sig: Signature,
    pub sig_exp: ExpirationDateSignature,
    pub kappa_k: Vec<G2Projective>,
    pub omega: Vec<CoinIndexSignature>,
    pub ss: Vec<G1Projective>,
    pub tt: Vec<G1Projective>,
    pub aa: Vec<G1Projective>,
    pub spend_value: u64,
    pub cc: G1Projective,
    pub zk_proof: SpendProof,
}

impl Payment {
    /// Checks the validity of the payment signature.
    ///
    /// This function performs two checks to ensure the payment signature is valid:
    /// - Verifies that the element `h` of the payment signature does not equal the identity.
    /// - Performs a bilinear pairing check involving the elements of the signature and the payment (`h`, `kappa`, and `s`).
    ///
    /// # Arguments
    ///
    /// * `params` - A reference to the system parameters required for the checks.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success if the signature is valid or an error if any check fails.
    ///
    /// # Errors
    ///
    /// An error is returned if:
    /// - The element `h` of the payment signature equals the identity.
    /// - The bilinear pairing check for `kappa` fails.
    ///
    pub fn check_signature_validity(&self, params: &Parameters) -> Result<()> {
        if bool::from(self.sig.0.is_identity()) {
            return Err(CompactEcashError::Spend(
                "The element h of the payment signature equals the identity".to_string(),
            ));
        }

        if !check_bilinear_pairing(
            &self.sig.0.to_affine(),
            &G2Prepared::from(self.kappa.to_affine()),
            &self.sig.1.to_affine(),
            params.grp().prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::Spend(
                "The bilinear check for kappa failed".to_string(),
            ));
        }
        Ok(())
    }

    /// Checks the validity of the expiration signature encoded in the payment given a spending date.
    /// If the spending date is within the allowed range before the expiration date, the check is successful.
    ///
    /// This function performs two checks to ensure the payment expiration signature is valid:
    /// - Verifies that the element `h` of the expiration signature does not equal the identity.
    /// - Performs a bilinear pairing check involving the elements of the expiration signature and the payment (`h`, `kappa_e`, and `s`).
    ///
    /// # Arguments
    ///
    /// * `params` - A reference to the system parameters required for the checks.
    /// * `verification_key` - The global verification key of the signing authorities.
    /// * `spend_date` - The date associated with the payment.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success if the expiration signature is valid or an error if any check fails.
    ///
    /// # Errors
    ///
    /// An error is returned if:
    /// - The element `h` of the payment expiration signature equals the identity.
    /// - The bilinear pairing check for `kappa_e` fails.
    ///
    pub fn check_exp_signature_validity(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        spend_date: Scalar,
    ) -> Result<()> {
        // Check if the element h of the payment expiration signature equals the identity.
        if bool::from(self.sig_exp.h.is_identity()) {
            return Err(CompactEcashError::ExpirationDate(
                "The element h of the payment expiration signature equals the identity".to_string(),
            ));
        }

        // Calculate m1 and m2 values.
        let m1: Scalar = spend_date;
        let m2: Scalar = constants::TYPE_EXP;

        // Perform a bilinear pairing check for kappa_e
        let combined_kappa_e = self.kappa_e
            + verification_key.beta_g2.get(1).unwrap() * m1
            + verification_key.beta_g2.get(2).unwrap() * m2;

        if !check_bilinear_pairing(
            &self.sig_exp.h.to_affine(),
            &G2Prepared::from(combined_kappa_e.to_affine()),
            &self.sig_exp.s.to_affine(),
            params.grp().prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::ExpirationDate(
                "The bilinear check for kappa_e failed".to_string(),
            ));
        }

        Ok(())
    }

    /// Checks that all serial numbers in the payment are unique.
    ///
    /// This function verifies that each serial number in the payment's serial number array (`ss`) is unique.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success if all serial numbers are unique or an error if any serial number is duplicated.
    ///
    /// # Errors
    ///
    /// An error is returned if not all serial numbers in the payment are unique.
    ///
    pub fn no_duplicate_serial_numbers(&self) -> Result<()> {
        let mut seen_serial_numbers = Vec::new();

        for serial_number in &self.ss {
            if seen_serial_numbers.contains(serial_number) {
                return Err(CompactEcashError::Spend(
                    "Not all serial numbers are unique".to_string(),
                ));
            }
            seen_serial_numbers.push(*serial_number);
        }

        Ok(())
    }

    /// Checks the validity of the coin index signature at a specific index.
    ///
    /// This function performs two checks to ensure the coin index signature at a given index (`k`) is valid:
    /// - Verifies that the element `h` of the coin index signature does not equal the identity.
    /// - Calculates a combined element for the bilinear pairing check involving `kappa_k`, and verifies the pairing with the coin index signature elements (`h`, `kappa_k`, and `s`).
    ///
    /// # Arguments
    ///
    /// * `params` - A reference to the system parameters required for the checks.
    /// * `verification_key` - The global verification key of the signing authorities.
    /// * `k` - The index at which to check the coin index signature.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success if the coin index signature is valid or an error if any check fails.
    ///
    /// # Errors
    ///
    /// An error is returned if:
    /// - The element `h` of the coin index signature at the specified index equals the identity.
    /// - The bilinear pairing check for `kappa_k` at the specified index fails.
    /// - The specified index is out of bounds for the coin index signatures array (`omega`).
    ///
    pub fn check_coin_index_signature(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        k: u64,
    ) -> Result<()> {
        if let Some(coin_idx_sign) = self.omega.get(k as usize) {
            if bool::from(coin_idx_sign.h.is_identity()) {
                return Err(CompactEcashError::Spend(
                    "The element h of the signature on index l equals the identity".to_string(),
                ));
            }
            let combined_kappa_k = self.kappa_k[k as usize].to_affine()
                + verification_key.beta_g2.get(1).unwrap() * constants::TYPE_IDX
                + verification_key.beta_g2.get(2).unwrap() * constants::TYPE_IDX;

            if !check_bilinear_pairing(
                &coin_idx_sign.h.to_affine(),
                &G2Prepared::from(combined_kappa_k.to_affine()),
                &coin_idx_sign.s.to_affine(),
                params.grp().prepared_miller_g2(),
            ) {
                return Err(CompactEcashError::Spend(
                    "The bilinear check for kappa_l failed".to_string(),
                ));
            }
        } else {
            return Err(CompactEcashError::Spend("Index out of bounds".to_string()));
        }
        Ok(())
    }

    /// Verifies the validity of a spend transaction, including signature checks,
    /// expiration date signature checks, serial number uniqueness, coin index signature checks,
    /// and zero-knowledge proof verification.
    ///
    /// # Arguments
    ///
    /// * `params` - The cryptographic parameters.
    /// * `verification_key` - The verification key used for validation.
    /// * `pay_info` - The pay information associated with the transaction.
    /// * `spend_date` - The date at which the spending transaction occurs.
    ///
    /// # Returns
    ///
    /// Returns `Ok(true)` if the spend transaction is valid; otherwise, returns an error.
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo,
        spend_date: Scalar,
    ) -> Result<bool> {
        // check if all serial numbers are different
        self.no_duplicate_serial_numbers()?;
        // Verify whether the payment signature and kappa are correct
        self.check_signature_validity(params)?;
        // Verify whether the expiration date signature and kappa_e are correct
        self.check_exp_signature_validity(params, verification_key, spend_date)?;

        // Compute pay_info hash for each coin
        let mut rr = Vec::with_capacity(self.spend_value as usize);
        for k in 0..self.spend_value {
            // Verify whether the coin indices signatures and kappa_k are correct
            self.check_coin_index_signature(params, verification_key, k)?;
            // Compute hashes R_k = H(payinfo, k)
            let rr_k = compute_pay_info_hash(pay_info, k);
            rr.push(rr_k);
        }
        // verify the zk proof
        let instance = SpendInstance {
            kappa: self.kappa,
            cc: self.cc,
            aa: self.aa.clone(),
            ss: self.ss.clone(),
            tt: self.tt.clone(),
            kappa_k: self.kappa_k.clone(),
            kappa_e: self.kappa_e,
        };

        // verify the zk-proof
        if !self.zk_proof.verify(
            params,
            &instance,
            verification_key,
            &rr,
            pay_info,
            self.spend_value,
        ) {
            return Err(CompactEcashError::Spend(
                "ZkProof verification failed".to_string(),
            ));
        }

        Ok(true)
    }

    pub fn serial_number_bs58(&self) -> String {
        SerialNumber {
            inner: self.ss.clone(),
        }
        .to_bs58()
    }

    pub fn has_serial_number(&self, serial_number_bs58: &str) -> Result<bool> {
        let serial_number = SerialNumber::try_from_bs58(serial_number_bs58)?;
        let ret = self.ss.eq(&serial_number.inner);
        Ok(ret)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.kappa.to_affine().to_compressed());
        bytes.extend_from_slice(&self.kappa_e.to_affine().to_compressed());
        bytes.extend_from_slice(&self.sig.to_bytes());
        bytes.extend_from_slice(&self.sig_exp.to_bytes());
        bytes.extend_from_slice(&self.spend_value.to_le_bytes());
        bytes.extend_from_slice(&self.cc.to_affine().to_compressed());

        let kappa_k_len = self.kappa_k.len();
        let kappa_k_len_bytes = kappa_k_len.to_le_bytes();
        bytes.extend_from_slice(&kappa_k_len_bytes);
        for kk in &self.kappa_k {
            bytes.extend_from_slice(&kk.to_affine().to_compressed());
        }

        let omega_len = self.omega.len();
        let omega_len_bytes = omega_len.to_le_bytes();
        bytes.extend_from_slice(&omega_len_bytes);
        for o in &self.omega {
            bytes.extend_from_slice(&o.to_bytes());
        }

        let ss_len = self.ss.len();
        let ss_len_bytes = ss_len.to_le_bytes();
        bytes.extend_from_slice(&ss_len_bytes);
        for s in &self.ss {
            bytes.extend_from_slice(&s.to_affine().to_compressed());
        }

        let tt_len = self.tt.len();
        let tt_len_bytes = tt_len.to_le_bytes();
        bytes.extend_from_slice(&tt_len_bytes);
        for t in &self.tt {
            bytes.extend_from_slice(&t.to_affine().to_compressed());
        }

        let aa_len = self.aa.len();
        let aa_len_bytes = aa_len.to_le_bytes();
        bytes.extend_from_slice(&aa_len_bytes);
        for a in &self.aa {
            bytes.extend_from_slice(&a.to_affine().to_compressed());
        }

        let zk_proof_bytes = self.zk_proof.to_bytes();
        bytes.extend_from_slice(&zk_proof_bytes);
        bytes
    }
}

impl TryFrom<&[u8]> for Payment {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Payment> {
        if bytes.len() < 848 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for Payment deserialization".to_string(),
            ));
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let kappa_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let kappa_e_bytes: [u8; 96] = bytes[96..192].try_into().unwrap();
        let kappa_e = try_deserialize_g2_projective(
            &kappa_e_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa_e".to_string()),
        )?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let sig_bytes: [u8; 96] = bytes[192..288].try_into().unwrap();
        let sig = Signature::try_from(sig_bytes.as_slice())?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let sig_exp_bytes: [u8; 96] = bytes[288..384].try_into().unwrap();
        let sig_exp = ExpirationDateSignature::try_from(sig_exp_bytes.as_slice())?;

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let spend_value_bytes: [u8; 8] = bytes[384..392].try_into().unwrap();
        let spend_value = u64::from_le_bytes(spend_value_bytes);

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let cc_bytes: [u8; 48] = bytes[392..440].try_into().unwrap();
        let cc = try_deserialize_g1_projective(
            &cc_bytes,
            CompactEcashError::Deserialization("Failed to deserialize cc".to_string()),
        )?;

        let mut idx = 440;
        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let kappa_k_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut kappa_k = Vec::with_capacity(kappa_k_len);
        for _ in 0..kappa_k_len {
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let kappa_k_bytes: [u8; 96] = bytes[idx..idx + 96].try_into().unwrap();
            let kappa_k_elem = try_deserialize_g2_projective(
                &kappa_k_bytes,
                CompactEcashError::Deserialization(
                    "Failed to deserialize kappa_k element".to_string(),
                ),
            )?;
            kappa_k.push(kappa_k_elem);
            idx += 96;
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let omega_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut omega = Vec::with_capacity(omega_len);
        for _ in 0..omega_len {
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let omega_bytes: [u8; 96] = bytes[idx..idx + 96].try_into().unwrap();
            let omega_elem = CoinIndexSignature::try_from(omega_bytes.as_slice())?;
            omega.push(omega_elem);
            idx += 96;
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let ss_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut ss = Vec::with_capacity(ss_len);
        for _ in 0..ss_len {
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let ss_bytes: [u8; 48] = bytes[idx..idx + 48].try_into().unwrap();
            let ss_elem = try_deserialize_g1_projective(
                &ss_bytes,
                CompactEcashError::Deserialization("Failed to deserialize ss element".to_string()),
            )?;
            ss.push(ss_elem);
            idx += 48;
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let tt_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut tt = Vec::with_capacity(tt_len);
        for _ in 0..tt_len {
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let tt_bytes: [u8; 48] = bytes[idx..idx + 48].try_into().unwrap();
            let tt_elem = try_deserialize_g1_projective(
                &tt_bytes,
                CompactEcashError::Deserialization("Failed to deserialize tt element".to_string()),
            )?;
            tt.push(tt_elem);
            idx += 48;
        }

        //SAFETY : slice to array conversion after a length check
        #[allow(clippy::unwrap_used)]
        let aa_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut aa = Vec::with_capacity(aa_len);
        for _ in 0..aa_len {
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let aa_bytes: [u8; 48] = bytes[idx..idx + 48].try_into().unwrap();
            let aa_elem = try_deserialize_g1_projective(
                &aa_bytes,
                CompactEcashError::Deserialization("Failed to deserialize aa element".to_string()),
            )?;
            aa.push(aa_elem);
            idx += 48;
        }

        // Deserialize the SpendProof struct
        let zk_proof_bytes = &bytes[idx..];
        let zk_proof = SpendProof::try_from(zk_proof_bytes)?;

        // Construct the Payment struct from the deserialized data
        let payment = Payment {
            kappa,
            kappa_e,
            sig,
            sig_exp,
            kappa_k,
            omega,
            ss,
            tt,
            aa,
            spend_value,
            cc,
            zk_proof,
        };

        Ok(payment)
    }
}

impl Bytable for Payment {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> std::result::Result<Self, CompactEcashError> {
        Self::try_from(slice)
    }
}

impl Base58 for Payment {}

pub struct SerialNumber {
    pub(crate) inner: Vec<G1Projective>,
}

impl SerialNumber {
    pub fn to_bytes(&self) -> Vec<u8> {
        let ss_len = self.inner.len();
        let mut bytes: Vec<u8> = Vec::with_capacity(ss_len * 48);
        for s in &self.inner {
            bytes.extend_from_slice(&s.to_affine().to_compressed());
        }
        bytes
    }
}

impl TryFrom<&[u8]> for SerialNumber {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() % 48 != 0 {
            return Err(
                CompactEcashError::Deserialization(
                    format!("Tried to deserialize blinded serial number with incorrect number of bytes, expected a multiple of 48, got {}", bytes.len()),
                ));
        }
        let inner_len = bytes.len() / 48;
        let mut inner = Vec::with_capacity(inner_len);
        let mut idx = 0;
        for _ in 0..inner_len {
            //SAFETY : slice to array conversion after a length check
            #[allow(clippy::unwrap_used)]
            let ss_bytes: [u8; 48] = bytes[idx..idx + 48].try_into().unwrap();
            let ss_elem = try_deserialize_g1_projective(
                &ss_bytes,
                CompactEcashError::Deserialization("Failed to deserialize ss element".to_string()),
            )?;
            inner.push(ss_elem);
            idx += 48;
        }

        Ok(SerialNumber { inner })
    }
}

impl Bytable for SerialNumber {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Self::try_from(slice)
    }
}

impl Base58 for SerialNumber {}
