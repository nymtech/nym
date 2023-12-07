use std::cell::Cell;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;

use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::expiration_date_signatures::{find_index, ExpirationDateSignature};
use crate::scheme::keygen::{PublicKeyUser, SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{CoinIndexSignature, GroupParameters, Parameters};
use crate::utils::{
    check_bilinear_pairing, hash_to_scalar, try_deserialize_g1_projective,
    try_deserialize_g2_projective, try_deserialize_scalar, Signature, SignerIndex,
};
use crate::Attribute;
use chrono::{Timelike, Utc};
use rand::{thread_rng, Rng};
use crate::constants;

pub mod aggregation;
pub mod expiration_date_signatures;
pub mod identify;
pub mod keygen;
pub mod setup;
pub mod withdrawal;

#[derive(Debug, Clone, PartialEq)]
pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
    expiration_date: Scalar,
}

impl PartialWallet {
    pub fn signature(&self) -> &Signature {
        &self.sig
    }
    pub fn v(&self) -> Scalar {
        self.v
    }
    pub fn index(&self) -> Option<SignerIndex> {
        self.idx
    }
    pub fn to_bytes(&self) -> [u8; 168] {
        let mut bytes = [0u8; 168];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..160].copy_from_slice(&self.expiration_date.to_bytes());
        // Check if idx is Some and copy its bytes if it exists
        if let Some(idx) = &self.idx {
            bytes[160..168].copy_from_slice(&idx.to_le_bytes());
        }
        bytes
    }
    pub fn expiration_date(&self) -> Scalar {
        self.expiration_date
    }
}

impl TryFrom<&[u8]> for PartialWallet {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<PartialWallet> {
        if bytes.len() != 168 {
            return Err(CompactEcashError::Deserialization(format!(
                "PartialWallet should be exactly 136 bytes, got {}",
                bytes.len()
            )));
        }

        let sig_bytes: &[u8; 96] = &bytes[..96].try_into().expect("Slice size != 96");
        let v_bytes: &[u8; 32] = &bytes[96..128].try_into().expect("Slice size != 32");
        let expiration_date_bytes = &bytes[128..160].try_into().expect("Slice size != 32");
        let idx_bytes: &[u8; 8] = &bytes[160..168].try_into().expect("Slice size != 8");

        let sig = Signature::try_from(sig_bytes.as_slice()).unwrap();
        let v = Scalar::from_bytes(&v_bytes).unwrap();
        let expiration_date = Scalar::from_bytes(&expiration_date_bytes).unwrap();
        let idx = None;
        if !idx_bytes.iter().all(|&x| x == 0) {
            let idx = Some(u64::from_le_bytes(*idx_bytes));
        }

        Ok(PartialWallet {
            sig,
            v,
            idx,
            expiration_date,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wallet {
    sig: Signature,
    v: Scalar,
    expiration_date: Scalar,
    pub l: Cell<u64>,
}

pub fn compute_payinfo_hash(pay_info: &PayInfo, k: u64) -> Scalar {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&pay_info.payinfo);
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

    pub fn to_bytes(&self) -> [u8; 168] {
        let mut bytes = [0u8; 168];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..160].copy_from_slice(&self.expiration_date.to_bytes());
        bytes[160..168].copy_from_slice(&self.l.get().to_le_bytes());
        bytes
    }
    fn up(&self) {
        self.l.set(self.l.get() + 1);
    }

    fn check_remaining_allowance(&self, params: &Parameters, spend_vv: u64) -> Result<()> {
        if self.l() + spend_vv > params.L() {
            Err(CompactEcashError::Spend(
                "The amount you want to spend exceeds remaining wallet allowance ".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    pub fn spend(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        bench_flag: bool,
        spend_vv: u64,
        valid_dates_signatures: Vec<ExpirationDateSignature>,
        coin_indices_signatures: Vec<CoinIndexSignature>,
        spend_date: Scalar,
    ) -> Result<(Payment, &Self)> {
        let grp_params = params.grp();
        let attributes = vec![sk_user.sk, self.v(), self.expiration_date()];
        // Check if we have enough remaining allowance in the wallet
        self.check_remaining_allowance(&params, spend_vv)?;
        // randomize wallet signature
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grp_params);

        // compute kappa (i.e., blinded attributes for show) to prove possession of the wallet signature
        let kappa = compute_kappa(
            &grp_params,
            &verification_key,
            &attributes,
            sign_blinding_factor,
        );

        // randomise the expiration date signature and compute kappa_e to prove possession of the expiration signature
        let date_signature_index = find_index(spend_date, self.expiration_date)?;
        let date_signature: ExpirationDateSignature = valid_dates_signatures
            .get(date_signature_index)
            .unwrap()
            .clone();
        // randomise the date signature
        let (date_signature_prime, date_sign_blinding_factor) =
            date_signature.randomise(&grp_params);
        // compute kappa_e to prove possession of the expiration signature
        let kappa_e: G2Projective = grp_params.gen2() * date_sign_blinding_factor
            + verification_key.alpha
            + verification_key.beta_g2.get(0).unwrap() * self.expiration_date();

        // pick random openings o_c and compute commitments C to v
        let o_c = grp_params.random_scalar();
        let cc = grp_params.gen1() * o_c + grp_params.gamma_idx(0).unwrap() * self.v();

        let mut aa: Vec<G1Projective> = Default::default();
        let mut ss: Vec<G1Projective> = Default::default();
        let mut tt: Vec<G1Projective> = Default::default();
        let mut rr: Vec<Scalar> = Default::default();
        let mut o_a: Vec<Scalar> = Default::default();
        let mut o_mu: Vec<Scalar> = Default::default();
        let mut mu: Vec<Scalar> = Default::default();
        let mut r_k_vec: Vec<Scalar> = Default::default();
        let mut kappa_k_vec: Vec<G2Projective> = Default::default();
        let mut sign_lk_prime_vec: Vec<Signature> = Default::default();
        let mut lk: Vec<Scalar> = Default::default();

        let mut coin_indices_signatures_prime: Vec<CoinIndexSignature> = Default::default();
        for k in 0..spend_vv {
            lk.push(Scalar::from(self.l() + k));

            // compute hashes R_k = H(payinfo, k)
            let rr_k = compute_payinfo_hash(&pay_info, k);
            rr.push(rr_k);

            let o_a_k = grp_params.random_scalar();
            o_a.push(o_a_k);
            let aa_k = grp_params.gen1() * o_a_k
                + grp_params.gamma_idx(0).unwrap() * Scalar::from(self.l() + k);
            aa.push(aa_k);

            // compute the serial numbers
            let ss_k = pseudorandom_f_delta_v(&grp_params, self.v(), self.l() + k);
            ss.push(ss_k);
            // compute the identification tags
            let tt_k = grp_params.gen1() * sk_user.sk
                + pseudorandom_f_g_v(&grp_params, self.v(), self.l() + k) * rr_k;
            tt.push(tt_k);

            // compute values mu, o_mu, lambda, o_lambda
            let mu_k: Scalar = (self.v() + Scalar::from(self.l() + k) + Scalar::from(1))
                .invert()
                .unwrap();
            mu.push(mu_k);

            let o_mu_k = ((o_a_k + o_c) * mu_k).neg();
            o_mu.push(o_mu_k);

            // randomise the coin indices signatures and compute kappa_k to prove possession of each signature
            // of the coin index
            let coin_sign: CoinIndexSignature =
                coin_indices_signatures.get(k as usize).unwrap().clone();
            let (coin_sign_prime, coin_sign_blinding_factor) = coin_sign.randomise(&grp_params);
            coin_indices_signatures_prime.push(coin_sign_prime);
            let kappa_k: G2Projective = grp_params.gen2() * coin_sign_blinding_factor
                + verification_key.alpha
                + verification_key.beta_g2.get(0).unwrap() * Scalar::from(self.l() + k);
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
            lk,
            o_a,
            mu,
            o_mu,
            r_k: r_k_vec,
            r_e: date_sign_blinding_factor,
            expiration_date: self.expiration_date,
        };

        let zk_proof = SpendProof::construct(
            &params,
            &spend_instance,
            &spend_witness,
            &verification_key,
            &rr,
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
            vv: spend_vv,
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
            self.l.set(current_l + spend_vv);
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

        let sig_bytes: &[u8; 96] = &bytes[..96].try_into().expect("Slice size != 96");
        let v_bytes: &[u8; 32] = &bytes[96..128].try_into().expect("Slice size != 32");
        let expiration_date_bytes: &[u8; 32] =
            &bytes[128..160].try_into().expect("Slice size != 32");
        let l_bytes: &[u8; 8] = &bytes[160..168].try_into().expect("Slice size != 8");

        let sig = Signature::try_from(sig_bytes.as_slice()).unwrap();
        let v = Scalar::from_bytes(&v_bytes).unwrap();
        let expiration_date = Scalar::from_bytes(&expiration_date_bytes).unwrap();
        let l = Cell::new(u64::from_le_bytes(*l_bytes));

        Ok(Wallet {
            sig,
            v,
            expiration_date,
            l,
        })
    }
}

pub fn pseudorandom_f_delta_v(params: &GroupParameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
    params.delta() * pow
}

pub fn pseudorandom_f_g_v(params: &GroupParameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).invert().unwrap();
    params.gen1() * pow
}

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

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct PayInfo {
    pub payinfo: [u8; 88],
}

impl PayInfo {
    pub fn generate_pay_info(provider_pk: PublicKeyUser) -> PayInfo {
        let mut payinfo = [0u8; 88];

        // Generating random bytes
        thread_rng().fill(&mut payinfo[..32]);

        // Adding timestamp bytes
        let timestamp = Utc::now().timestamp();
        payinfo[32..40].copy_from_slice(&timestamp.to_be_bytes());

        // Adding provider public key bytes
        let ppk_bytes = provider_pk.pk.to_affine().to_compressed();
        payinfo[40..].copy_from_slice(&ppk_bytes);

        PayInfo { payinfo }
    }
}
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
    pub vv: u64,
    pub cc: G1Projective,
    pub zk_proof: SpendProof,
}

impl Payment {
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo,
        spend_date: Scalar,
    ) -> Result<bool> {
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

        let m1: Scalar = spend_date;
        let m2 = constants::TYPE_EXP;

        if bool::from(self.sig_exp.h.is_identity()) {
            return Err(CompactEcashError::Spend(
                "The element h of the payment expiration signature equals the identity".to_string(),
            ));
        }

        let tmp = self.kappa_e
        + verification_key.beta_g2.get(0).unwrap() * m1
        + verification_key.beta_g2.get(1).unwrap() * Scalar::from_bytes(&m2).unwrap();

        // if !check_bilinear_pairing(
        //     &self.sig_exp.h.to_affine(),
        //     &G2Prepared::from(tmp.to_affine()),
        //     &self.sig_exp.s.to_affine(),
        //     params.grp().prepared_miller_g2(),
        // ) {
        //     return Err(CompactEcashError::Spend(
        //         "The bilinear check for kappa_e failed".to_string(),
        //     ));
        // }

        // check if all serial numbers are different
        for k in 0..self.vv {
            if let Some(coin_idx_sign) = self.omega.get(k as usize) {
                if bool::from(coin_idx_sign.h.is_identity()) {
                    return Err(CompactEcashError::Spend(
                        "The element h of the signature on index l equals the identity".to_string(),
                    ));
                }
                let tmp2 = self.kappa_k[k as usize].to_affine()
                    + verification_key.beta_g2.get(1).unwrap() * Scalar::from_bytes(&constants::TYPE_IDX).unwrap()
                    + verification_key.beta_g2.get(2).unwrap() * Scalar::from_bytes(&constants::TYPE_IDX).unwrap();

                if !check_bilinear_pairing(
                    &coin_idx_sign.h.to_affine(),
                    &G2Prepared::from(tmp2.to_affine()),
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
        }

        let mut rr = Vec::with_capacity(self.vv as usize);
        for k in 0..self.vv {
            // compute hashes R_k = H(payinfo, k)
            let rr_k = compute_payinfo_hash(&pay_info, k);
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
            kappa_e: self.kappa_e.clone(),
        };

        // verify the zk-proof
        // if !self
        //     .zk_proof
        //     .verify(&params, &instance, &verification_key)
        // {
        //     return Err(CompactEcashError::Spend(
        //         "ZkProof verification failed".to_string(),
        //     ));
        // }

        Ok(true)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let kappa_bytes: [u8; 96] = self.kappa.to_affine().to_compressed();
        let kappa_e_bytes: [u8; 96] = self.kappa_e.to_affine().to_compressed();
        let sig_bytes = self.sig.to_bytes();
        let sig_exp_bytes = self.sig_exp.to_bytes();
        let vv_bytes: [u8; 8] = self.vv.to_le_bytes();
        let cc_bytes: [u8; 48] = self.cc.to_affine().to_compressed();
        let kappa_k_len = self.kappa_k.len();
        let omega_len = self.omega.len();
        let ss_len = self.ss.len();
        let tt_len = self.tt.len();
        let aa_len = self.aa.len();
        let zk_proof_bytes = self.zk_proof.to_bytes();
        let zk_proof_bytes_len = self.zk_proof.to_bytes().len();

        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend_from_slice(&kappa_bytes);
        bytes.extend_from_slice(&kappa_e_bytes);
        bytes.extend_from_slice(&sig_bytes);
        bytes.extend_from_slice(&sig_exp_bytes);
        bytes.extend_from_slice(&vv_bytes);
        bytes.extend_from_slice(&cc_bytes);

        let kappa_k_len_bytes = kappa_k_len.to_le_bytes();
        bytes.extend_from_slice(&kappa_k_len_bytes);
        for kk in &self.kappa_k {
            bytes.extend_from_slice(&kk.to_affine().to_compressed());
        }

        let omega_len_bytes = omega_len.to_le_bytes();
        bytes.extend_from_slice(&omega_len_bytes);
        for o in &self.omega {
            bytes.extend_from_slice(&o.to_bytes());
        }

        let ss_len_bytes = ss_len.to_le_bytes();
        bytes.extend_from_slice(&ss_len_bytes);
        for s in &self.ss {
            bytes.extend_from_slice(&s.to_affine().to_compressed());
        }

        let tt_len_bytes = tt_len.to_le_bytes();
        bytes.extend_from_slice(&tt_len_bytes);
        for t in &self.tt {
            bytes.extend_from_slice(&t.to_affine().to_compressed());
        }

        let aa_len_bytes = aa_len.to_le_bytes();
        bytes.extend_from_slice(&aa_len_bytes);
        for a in &self.aa {
            bytes.extend_from_slice(&a.to_affine().to_compressed());
        }

        bytes.extend_from_slice(&zk_proof_bytes);
        bytes
    }
}

impl TryFrom<&[u8]> for Payment {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Payment> {
        if bytes.len() < 816 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for Payment deserialization".to_string(),
            ));
        }

        let kappa_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;

        let kappa_e_bytes: [u8; 96] = bytes[96..192].try_into().unwrap();
        let kappa_e = try_deserialize_g2_projective(
            &kappa_e_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa_e".to_string()),
        )?;

        let sig_bytes: [u8; 96] = bytes[192..288].try_into().unwrap();
        let sig = Signature::try_from(sig_bytes.as_slice())?;

        let sig_exp_bytes: [u8; 96] = bytes[288..384].try_into().unwrap();
        let sig_exp = ExpirationDateSignature::try_from(sig_exp_bytes.as_slice())?;

        let vv_bytes: [u8; 8] = bytes[384..392].try_into().unwrap();
        let vv = u64::from_le_bytes(vv_bytes);

        let cc_bytes: [u8; 48] = bytes[392..440].try_into().unwrap();
        let cc = try_deserialize_g1_projective(
            &cc_bytes,
            CompactEcashError::Deserialization("Failed to deserialize cc".to_string()),
        )?;

        let mut idx = 440;
        let kappa_k_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut kappa_k = Vec::with_capacity(kappa_k_len);
        for _ in 0..kappa_k_len {
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

        let omega_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut omega = Vec::with_capacity(omega_len);
        for _ in 0..omega_len {
            let omega_bytes: [u8; 96] = bytes[idx..idx + 96].try_into().unwrap();
            let omega_elem = CoinIndexSignature::try_from(omega_bytes.as_slice())?;
            omega.push(omega_elem);
            idx += 96;
        }

        let ss_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut ss = Vec::with_capacity(ss_len);
        for _ in 0..ss_len {
            let ss_bytes: [u8; 48] = bytes[idx..idx + 48].try_into().unwrap();
            let ss_elem = try_deserialize_g1_projective(
                &ss_bytes,
                CompactEcashError::Deserialization("Failed to deserialize ss element".to_string()),
            )?;
            ss.push(ss_elem);
            idx += 48;
        }

        let tt_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut tt = Vec::with_capacity(tt_len);
        for _ in 0..tt_len {
            let tt_bytes: [u8; 48] = bytes[idx..idx + 48].try_into().unwrap();
            let tt_elem = try_deserialize_g1_projective(
                &tt_bytes,
                CompactEcashError::Deserialization("Failed to deserialize tt element".to_string()),
            )?;
            tt.push(tt_elem);
            idx += 48;
        }

        let aa_len = u64::from_le_bytes(bytes[idx..idx + 8].try_into().unwrap()) as usize;
        idx += 8;
        let mut aa = Vec::with_capacity(aa_len);
        for _ in 0..aa_len {
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
            vv,
            cc,
            zk_proof,
        };

        Ok(payment)
    }
}
