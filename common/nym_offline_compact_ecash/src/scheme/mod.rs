// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::common_types::{Signature, SignerIndex};
use crate::error::{CompactEcashError, Result};
use crate::helpers::{date_scalar, type_scalar};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::coin_indices_signatures::CoinIndexSignature;
use crate::scheme::expiration_date_signatures::{find_index, ExpirationDateSignature};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::traits::Bytable;
use crate::utils::{
    batch_verify_signatures, check_bilinear_pairing, hash_to_scalar, try_deserialize_scalar,
};
use crate::{constants, ecash_group_parameters};
use crate::{Base58, EncodedDate, EncodedTicketType};
use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Borrow;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub mod aggregation;
pub mod coin_indices_signatures;
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
    t_type: Scalar,
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
    pub fn t_type(&self) -> Scalar {
        self.t_type
    }

    /// Converts the `PartialWallet` to a fixed-size byte array.
    ///
    /// The resulting byte array has a length of 200 bytes and contains serialized
    /// representations of the `Signature` (`sig`), scalar value (`v`),
    /// expiration date (`expiration_date`), and `idx` fields of the `PartialWallet` struct.
    ///
    /// # Returns
    ///
    /// A fixed-size byte array (`[u8; 200]`) representing the serialized form of the `PartialWallet`.
    ///
    pub fn to_bytes(&self) -> [u8; 200] {
        let mut bytes = [0u8; 200];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..160].copy_from_slice(&self.expiration_date.to_bytes());
        bytes[160..192].copy_from_slice(&self.t_type.to_bytes());
        bytes[192..200].copy_from_slice(&self.idx.to_le_bytes());
        bytes
    }

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
    pub fn from_bytes(bytes: &[u8]) -> Result<PartialWallet> {
        const SIGNATURE_BYTES: usize = 96;
        const V_BYTES: usize = 32;
        const EXPIRATION_DATE_BYTES: usize = 32;
        const T_TYPE_BYTES: usize = 32;
        const IDX_BYTES: usize = 8;
        const EXPECTED_LENGTH: usize =
            SIGNATURE_BYTES + V_BYTES + EXPIRATION_DATE_BYTES + T_TYPE_BYTES + IDX_BYTES;

        if bytes.len() != EXPECTED_LENGTH {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "PartialWallet".into(),
                expected: EXPECTED_LENGTH,
                actual: bytes.len(),
            });
        }

        let mut j = 0;

        let sig = Signature::try_from(&bytes[j..j + SIGNATURE_BYTES])?;
        j += SIGNATURE_BYTES;

        //SAFETY: slice to array after length check
        #[allow(clippy::unwrap_used)]
        let v_bytes = bytes[j..j + V_BYTES].try_into().unwrap();
        let v = try_deserialize_scalar(v_bytes)?;
        j += V_BYTES;

        //SAFETY: slice to array after length check
        #[allow(clippy::unwrap_used)]
        let expiration_date_bytes = bytes[j..j + EXPIRATION_DATE_BYTES].try_into().unwrap();
        let expiration_date = try_deserialize_scalar(expiration_date_bytes)?;
        j += EXPIRATION_DATE_BYTES;
        //SAFETY: slice to array after length check
        #[allow(clippy::unwrap_used)]
        let t_type_bytes = bytes[j..j + T_TYPE_BYTES].try_into().unwrap();
        let t_type = try_deserialize_scalar(t_type_bytes)?;
        j += T_TYPE_BYTES;

        //SAFETY: slice to array after length check
        #[allow(clippy::unwrap_used)]
        let idx_bytes = bytes[j..].try_into().unwrap();
        let idx = u64::from_le_bytes(idx_bytes);

        Ok(PartialWallet {
            sig,
            v,
            idx,
            expiration_date,
            t_type,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Zeroize, Serialize, Deserialize)]
pub struct Wallet {
    /// The cryptographic materials required for producing spending proofs and payments.
    signatures: WalletSignatures,

    /// Also known as `l` parameter in the paper
    tickets_spent: u64,
}

impl Wallet {
    pub fn new(signatures: WalletSignatures, tickets_spent: u64) -> Self {
        Wallet {
            signatures,
            tickets_spent,
        }
    }

    pub fn into_wallet_signatures(self) -> WalletSignatures {
        self.into()
    }

    pub fn to_bytes(&self) -> [u8; WalletSignatures::SERIALISED_SIZE + 8] {
        let mut bytes = [0u8; WalletSignatures::SERIALISED_SIZE + 8];
        bytes[0..WalletSignatures::SERIALISED_SIZE].copy_from_slice(&self.signatures.to_bytes());
        bytes[WalletSignatures::SERIALISED_SIZE..]
            .copy_from_slice(&self.tickets_spent.to_be_bytes());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Wallet> {
        if bytes.len() != WalletSignatures::SERIALISED_SIZE + 8 {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "Wallet".into(),
                expected: WalletSignatures::SERIALISED_SIZE + 8,
                actual: bytes.len(),
            });
        }

        //SAFETY : slice to array conversions after a length check
        #[allow(clippy::unwrap_used)]
        let tickets_bytes = bytes[WalletSignatures::SERIALISED_SIZE..]
            .try_into()
            .unwrap();

        let signatures = WalletSignatures::from_bytes(&bytes[..WalletSignatures::SERIALISED_SIZE])?;
        let tickets_spent = u64::from_be_bytes(tickets_bytes);

        Ok(Wallet {
            signatures,
            tickets_spent,
        })
    }

    pub fn ensure_allowance(
        params: &Parameters,
        tickets_spent: u64,
        spend_value: u64,
    ) -> Result<()> {
        if tickets_spent + spend_value > params.get_total_coins() {
            Err(CompactEcashError::SpendExceedsAllowance {
                spending: spend_value,
                remaining: params.get_total_coins() - tickets_spent,
            })
        } else {
            Ok(())
        }
    }

    pub fn check_remaining_allowance(&self, params: &Parameters, spend_value: u64) -> Result<()> {
        Self::ensure_allowance(params, self.tickets_spent, spend_value)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spend(
        &mut self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        spend_value: u64,
        valid_dates_signatures: &[ExpirationDateSignature],
        coin_indices_signatures: &[CoinIndexSignature],
        spend_date_timestamp: EncodedDate,
    ) -> Result<Payment> {
        self.check_remaining_allowance(params, spend_value)?;

        // produce payment
        let payment = self.signatures.spend(
            params,
            verification_key,
            sk_user,
            pay_info,
            self.tickets_spent,
            spend_value,
            valid_dates_signatures,
            coin_indices_signatures,
            spend_date_timestamp,
        )?;

        // update the ticket counter
        self.tickets_spent += spend_value;
        Ok(payment)
    }
}

impl From<Wallet> for WalletSignatures {
    fn from(value: Wallet) -> Self {
        value.signatures
    }
}

/// The struct represents a wallet with essential components for a payment transaction.
///
/// A `Wallet` includes a Pointcheval-Sanders signature (`sig`),
/// a scalar value (`v`) representing the wallet's secret, an optional
/// an expiration date (`expiration_date`)
/// and an u64 ('l') indicating the total number of spent coins.
///
#[derive(Debug, Clone, PartialEq, Zeroize, ZeroizeOnDrop, Serialize, Deserialize)]
pub struct WalletSignatures {
    #[zeroize(skip)]
    sig: Signature,
    v: Scalar,
    expiration_date_timestamp: EncodedDate,
    t_type: EncodedTicketType,
}

impl WalletSignatures {
    pub fn with_tickets_spent(self, tickets_spent: u64) -> Wallet {
        Wallet {
            signatures: self,
            tickets_spent,
        }
    }

    pub fn new_wallet(self) -> Wallet {
        self.with_tickets_spent(0)
    }

    pub fn encoded_expiration_date(&self) -> Scalar {
        date_scalar(self.expiration_date_timestamp)
    }
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

impl WalletSignatures {
    // signature size (96) + secret size (32) + expiration size (4) + t_type (1)
    pub const SERIALISED_SIZE: usize = 133;

    pub fn signature(&self) -> &Signature {
        &self.sig
    }

    /// Converts the `WalletSignatures` to a fixed-size byte array.
    ///
    /// The resulting byte array has a length of 168 bytes and contains serialized
    /// representations of the `Signature` (`sig`), scalar value (`v`), and
    /// expiration date (`expiration_date`) fields of the `WalletSignatures` struct.
    ///
    /// # Returns
    ///
    /// A fixed-size byte array (`[u8; 136]`) representing the serialized form of the `Wallet`.
    ///
    pub fn to_bytes(&self) -> [u8; Self::SERIALISED_SIZE] {
        let mut bytes = [0u8; Self::SERIALISED_SIZE];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..132].copy_from_slice(&self.expiration_date_timestamp.to_be_bytes());
        bytes[132] = self.t_type;
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<WalletSignatures> {
        if bytes.len() != Self::SERIALISED_SIZE {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "WalletSignatures".into(),
                expected: Self::SERIALISED_SIZE,
                actual: bytes.len(),
            });
        }
        //SAFETY : slice to array conversions after a length check
        #[allow(clippy::unwrap_used)]
        let sig_bytes: &[u8; 96] = &bytes[..96].try_into().unwrap();

        #[allow(clippy::unwrap_used)]
        let v_bytes: &[u8; 32] = &bytes[96..128].try_into().unwrap();

        #[allow(clippy::unwrap_used)]
        let expiration_date_bytes = bytes[128..132].try_into().unwrap();

        let sig = Signature::try_from(sig_bytes.as_slice())?;
        let v = Scalar::from_bytes(v_bytes).unwrap();
        let expiration_date_timestamp = EncodedDate::from_be_bytes(expiration_date_bytes);
        let t_type = bytes[132];

        Ok(WalletSignatures {
            sig,
            v,
            expiration_date_timestamp,
            t_type,
        })
    }

    /// Performs a spending operation with the given parameters, updating the wallet and generating a payment.
    ///
    /// # Arguments
    ///
    /// * `verification_key` - The global verification key.
    /// * `sk_user` - The secret key of the user who wants to spend from their wallet.
    /// * `pay_info` - Unique information related to the payment.
    /// * `current_tickets_spent` - The total number of tickets already spent in the associated wallet.
    /// * `spend_value` - The amount to spend from the wallet.
    /// * `valid_dates_signatures` - A list of **SORTED** signatures on valid dates during which we can spend from the wallet.
    /// * `coin_indices_signatures` - A list of **SORTED** signatures on coin indices.
    /// * `spend_date` - The date on which the spending occurs, expressed as unix timestamp.
    ///
    /// # Returns
    ///
    /// A tuple containing the generated payment and a reference to the updated wallet, or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn spend<BI, BE>(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        current_tickets_spent: u64,
        spend_value: u64,
        valid_dates_signatures: &[BE],
        coin_indices_signatures: &[BI],
        spend_date_timestamp: EncodedDate,
    ) -> Result<Payment>
    where
        BI: Borrow<CoinIndexSignature>,
        BE: Borrow<ExpirationDateSignature>,
    {
        // Extract group parameters
        let grp_params = params.grp();

        if verification_key.beta_g2.is_empty() {
            return Err(CompactEcashError::VerificationKeyTooShort);
        }

        if valid_dates_signatures.len() != constants::CRED_VALIDITY_PERIOD_DAYS as usize {
            return Err(CompactEcashError::InsufficientNumberOfExpirationSignatures);
        }

        if coin_indices_signatures.len() != params.get_total_coins() as usize {
            return Err(CompactEcashError::InsufficientNumberOfIndexSignatures);
        }

        Wallet::ensure_allowance(params, current_tickets_spent, spend_value)?;

        // Wallet attributes needed for spending
        let attributes = [&sk_user.sk, &self.v, &self.encoded_expiration_date()];

        // Randomize wallet signature
        let (signature_prime, sign_blinding_factor) = self.signature().blind_and_randomise();

        // compute kappa (i.e., blinded attributes for show) to prove possession of the wallet signature
        let kappa = compute_kappa(
            grp_params,
            verification_key,
            &attributes,
            sign_blinding_factor,
        );

        // Randomise the expiration date signature for the date when we want to perform the spending, and compute kappa_e to prove possession of
        // the expiration signature
        let date_signature_index =
            find_index(spend_date_timestamp, self.expiration_date_timestamp)?;

        //SAFETY : find_index eiter returns a valid index or an error. The unwrap is therefore fine
        #[allow(clippy::unwrap_used)]
        let date_signature = valid_dates_signatures
            .get(date_signature_index)
            .unwrap()
            .borrow();
        let (date_signature_prime, date_sign_blinding_factor) =
            date_signature.blind_and_randomise();
        // compute kappa_e to prove possession of the expiration signature
        //SAFETY: we checked that verification beta_g2 isn't empty
        #[allow(clippy::unwrap_used)]
        let kappa_e: G2Projective = grp_params.gen2() * date_sign_blinding_factor
            + verification_key.alpha
            + verification_key.beta_g2.first().unwrap() * self.encoded_expiration_date();

        // pick random openings o_c and compute commitments C to v (wallet secret)
        let o_c = grp_params.random_scalar();
        //SAFETY: grp_params is static with length 3
        #[allow(clippy::unwrap_used)]
        let cc = grp_params.gen1() * o_c + grp_params.gamma_idx(1).unwrap() * self.v;

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
            let lk = current_tickets_spent + k;
            lk_vec.push(Scalar::from(lk));

            // compute hashes R_k = H(payinfo, k)
            let rr_k = compute_pay_info_hash(pay_info, k);
            rr.push(rr_k);

            let o_a_k = grp_params.random_scalar();
            o_a.push(o_a_k);
            //SAFETY: grp_params is static with length 3
            #[allow(clippy::unwrap_used)]
            let aa_k =
                grp_params.gen1() * o_a_k + grp_params.gamma_idx(1).unwrap() * Scalar::from(lk);
            aa.push(aa_k);

            // compute the serial numbers
            let ss_k = pseudorandom_f_delta_v(grp_params, &self.v, lk)?;
            ss.push(ss_k);
            // compute the identification tags
            let tt_k = grp_params.gen1() * sk_user.sk
                + pseudorandom_f_g_v(grp_params, &self.v, lk)? * rr_k;
            tt.push(tt_k);

            // compute values mu, o_mu, lambda, o_lambda
            let maybe_mu_k: Option<Scalar> = (self.v + Scalar::from(lk) + Scalar::from(1))
                .invert()
                .into();
            let mu_k = maybe_mu_k.ok_or(CompactEcashError::UnluckiestError)?;
            mu.push(mu_k);

            let o_mu_k = ((o_a_k + o_c) * mu_k).neg();
            o_mu.push(o_mu_k);

            // Randomize the coin index signatures and compute kappa_k to prove possession of each coin's signature
            // This involves iterating over the signatures corresponding to the coins we want to spend in this payment.
            //SAFETY : Earlier `ensure_allowance` ensures we don't do out of of bound here
            #[allow(clippy::unwrap_used)]
            let coin_sign = coin_indices_signatures.get(lk as usize).unwrap().borrow();
            let (coin_sign_prime, coin_sign_blinding_factor) = coin_sign.blind_and_randomise();
            coin_indices_signatures_prime.push(coin_sign_prime);
            //SAFETY: we checked that verification beta_g2 isn't empty
            #[allow(clippy::unwrap_used)]
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
            attributes: &attributes,
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
            &spend_instance,
            &spend_witness,
            verification_key,
            &rr,
            pay_info,
            spend_value,
        );

        // output pay
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
            t_type: self.t_type,
            zk_proof,
        };

        Ok(pay)
    }
}

fn pseudorandom_f_delta_v(params: &GroupParameters, v: &Scalar, l: u64) -> Result<G1Projective> {
    let maybe_pow: Option<Scalar> = (v + Scalar::from(l) + Scalar::from(1)).invert().into();
    Ok(params.delta() * maybe_pow.ok_or(CompactEcashError::UnluckiestError)?)
}

fn pseudorandom_f_g_v(params: &GroupParameters, v: &Scalar, l: u64) -> Result<G1Projective> {
    let maybe_pow: Option<Scalar> = (v + Scalar::from(l) + Scalar::from(1)).invert().into();
    Ok(params.gen1() * maybe_pow.ok_or(CompactEcashError::UnluckiestError)?)
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
/// * `blinding_factor` - The blinding factor used to randomise the wallet's signature.
///
/// # Returns
///
/// A `G2Projective` element representing the computed value of kappa.
///
fn compute_kappa(
    params: &GroupParameters,
    verification_key: &VerificationKeyAuth,
    attributes: &[&Scalar],
    blinding_factor: Scalar,
) -> G2Projective {
    params.gen2() * blinding_factor
        + verification_key.alpha
        + attributes
            .iter()
            .zip(verification_key.beta_g2.iter())
            .map(|(&priv_attr, beta_i)| beta_i * priv_attr)
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
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub struct PayInfo {
    pub pay_info_bytes: [u8; 72],
}

impl Serialize for PayInfo {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.pay_info_bytes.to_vec().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PayInfo {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let pay_info_bytes = <Vec<u8>>::deserialize(deserializer)?;
        Ok(PayInfo {
            pay_info_bytes: pay_info_bytes
                .try_into()
                .map_err(|_| serde::de::Error::custom("invalid pay info bytes"))?,
        })
    }
}

impl Bytable for PayInfo {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.pay_info_bytes.to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> std::result::Result<Self, CompactEcashError> {
        if slice.len() != 72 {
            return Err(CompactEcashError::DeserializationLengthMismatch {
                type_name: "PayInfo".into(),
                expected: 72,
                actual: slice.len(),
            });
        }
        //safety : we checked that slices length is exactly 72, hence this unwrap won't fail
        #[allow(clippy::unwrap_used)]
        Ok(Self {
            pay_info_bytes: slice.try_into().unwrap(),
        })
    }
}

impl Base58 for PayInfo {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    pub t_type: EncodedTicketType,
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
    pub fn check_signature_validity(&self, verification_key: &VerificationKeyAuth) -> Result<()> {
        let params = ecash_group_parameters();
        if bool::from(self.sig.h.is_identity()) {
            return Err(CompactEcashError::SpendSignaturesValidity);
        }

        let kappa_type = self.kappa + verification_key.beta_g2[3] * type_scalar(self.t_type);
        if !check_bilinear_pairing(
            &self.sig.h.to_affine(),
            &G2Prepared::from(kappa_type.to_affine()),
            &self.sig.s.to_affine(),
            params.prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::SpendSignaturesValidity);
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
        verification_key: &VerificationKeyAuth,
        spend_date: Scalar,
    ) -> Result<()> {
        let grp_params = ecash_group_parameters();
        // Check if the element h of the payment expiration signature equals the identity.
        if bool::from(self.sig_exp.h.is_identity()) {
            return Err(CompactEcashError::ExpirationDateSignatureValidity);
        }

        if verification_key.beta_g2.len() < 3 {
            return Err(CompactEcashError::VerificationKeyTooShort);
        }

        // Calculate m1 and m2 values.
        let m1: Scalar = spend_date;
        let m2: Scalar = constants::TYPE_EXP;

        // Perform a bilinear pairing check for kappa_e
        //SAFETY: we checked the size of beta_G2 earlier
        let combined_kappa_e =
            self.kappa_e + verification_key.beta_g2[1] * m1 + verification_key.beta_g2[2] * m2;

        if !check_bilinear_pairing(
            &self.sig_exp.h.to_affine(),
            &G2Prepared::from(combined_kappa_e.to_affine()),
            &self.sig_exp.s.to_affine(),
            grp_params.prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::ExpirationDateSignatureValidity);
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
                return Err(CompactEcashError::SpendDuplicateSerialNumber);
            }
            seen_serial_numbers.push(*serial_number);
        }

        Ok(())
    }

    // /// Checks the validity of the coin index signature at a specific index.
    // ///
    // /// This function performs two checks to ensure the coin index signature at a given index (`k`) is valid:
    // /// - Verifies that the element `h` of the coin index signature does not equal the identity.
    // /// - Calculates a combined element for the bilinear pairing check involving `kappa_k`, and verifies the pairing with the coin index signature elements (`h`, `kappa_k`, and `s`).
    // ///
    // /// # Arguments
    // ///
    // /// * `verification_key` - The global verification key of the signing authorities.
    // /// * `k` - The index at which to check the coin index signature.
    // ///
    // /// # Returns
    // ///
    // /// A `Result` indicating success if the coin index signature is valid or an error if any check fails.
    // ///
    // /// # Errors
    // ///
    // /// An error is returned if:
    // /// - The element `h` of the coin index signature at the specified index equals the identity.
    // /// - The bilinear pairing check for `kappa_k` at the specified index fails.
    // /// - The specified index is out of bounds for the coin index signatures array (`omega`).
    // ///
    // pub fn check_coin_index_signature(
    //     &self,
    //     verification_key: &VerificationKeyAuth,
    //     k: u64,
    // ) -> Result<()> {
    //     if let Some(coin_idx_sign) = self.omega.get(k as usize) {
    //         if bool::from(coin_idx_sign.h.is_identity()) {
    //             return Err(CompactEcashError::SpendSignaturesVerification);
    //         }
    //         if verification_key.beta_g2.len() < 3 {
    //             return Err(CompactEcashError::VerificationKeyTooShort);
    //         }
    //         //SAFETY: we checked the size of beta_G2 earlier
    //         #[allow(clippy::unwrap_used)]
    //         let combined_kappa_k = self.kappa_k[k as usize].to_affine()
    //             + verification_key.beta_g2.get(1).unwrap() * constants::TYPE_IDX
    //             + verification_key.beta_g2.get(2).unwrap() * constants::TYPE_IDX;
    //
    //         if !check_bilinear_pairing(
    //             &coin_idx_sign.h.to_affine(),
    //             &G2Prepared::from(combined_kappa_k.to_affine()),
    //             &coin_idx_sign.s.to_affine(),
    //             ecash_group_parameters().prepared_miller_g2(),
    //         ) {
    //             return Err(CompactEcashError::SpendSignaturesVerification);
    //         }
    //     } else {
    //         return Err(CompactEcashError::SpendSignaturesVerification);
    //     }
    //     Ok(())
    // }

    /// Checks the validity of all coin index signatures available.
    pub fn batch_check_coin_index_signatures(
        &self,
        verification_key: &VerificationKeyAuth,
    ) -> Result<()> {
        if verification_key.beta_g2.len() < 3 {
            return Err(CompactEcashError::VerificationKeyTooShort);
        }

        if self.omega.len() != self.kappa_k.len() {
            return Err(CompactEcashError::SpendSignaturesVerification);
        }

        let partially_signed = verification_key.beta_g2[1] * constants::TYPE_IDX
            + verification_key.beta_g2[2] * constants::TYPE_IDX;

        let mut pairing_terms = Vec::with_capacity(self.omega.len());
        for (sig, kappa_k) in self.omega.iter().zip(self.kappa_k.iter()) {
            pairing_terms.push((sig, partially_signed + kappa_k))
        }

        if !batch_verify_signatures(pairing_terms.iter()) {
            return Err(CompactEcashError::SpendSignaturesVerification);
        }
        Ok(())
    }

    /// Checks the validity of the attached zk proof of spending.
    pub fn verify_spend_proof(
        &self,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo,
    ) -> Result<()> {
        // Compute pay_info hash for each coin
        let mut rr = Vec::with_capacity(self.spend_value as usize);
        for k in 0..self.spend_value {
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
        if !self
            .zk_proof
            .verify(&instance, verification_key, &rr, pay_info, self.spend_value)
        {
            return Err(CompactEcashError::SpendZKProofVerification);
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
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo,
        spend_date: EncodedDate,
    ) -> Result<()> {
        // check if all serial numbers are different
        self.no_duplicate_serial_numbers()?;
        // verify the zk proof
        self.verify_spend_proof(verification_key, pay_info)?;
        // Verify whether the payment signature and kappa are correct
        self.check_signature_validity(verification_key)?;
        // Verify whether the expiration date signature and kappa_e are correct
        self.check_exp_signature_validity(verification_key, date_scalar(spend_date))?;
        // Verify whether the coin indices signatures and kappa_k are correct
        self.batch_check_coin_index_signatures(verification_key)?;

        Ok(())
    }

    pub fn encoded_serial_number(&self) -> Vec<u8> {
        SerialNumberRef { inner: &self.ss }.to_bytes()
    }

    pub fn serial_number_bs58(&self) -> String {
        SerialNumberRef { inner: &self.ss }.to_bs58()
    }

    // pub fn has_serial_number(&self, serial_number_bs58: &str) -> Result<bool> {
    //     let serial_number = SerialNumberRef::try_from_bs58(serial_number_bs58)?;
    //     let ret = self.ss.eq(&serial_number.inner);
    //     Ok(ret)
    // }
}

pub struct SerialNumberRef<'a> {
    pub(crate) inner: &'a [G1Projective],
}

impl<'a> SerialNumberRef<'a> {
    pub fn to_bytes(&self) -> Vec<u8> {
        let ss_len = self.inner.len();
        let mut bytes: Vec<u8> = Vec::with_capacity(ss_len * 48);
        for s in self.inner {
            bytes.extend_from_slice(&s.to_affine().to_compressed());
        }
        bytes
    }

    pub fn to_bs58(&self) -> String {
        bs58::encode(self.to_bytes()).into_string()
    }
}
