use std::cell::Cell;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;

use crate::Attribute;
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{
    check_bilinear_pairing, hash_to_scalar, Signature, SignerIndex,
    try_deserialize_g1_projective, try_deserialize_scalar, try_deserialize_g2_projective,
};

pub mod aggregation;
pub mod identify;
pub mod keygen;
pub mod setup;
pub mod withdrawal;

#[derive(Debug, Clone, PartialEq)]
pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
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
    pub fn to_bytes(&self) -> [u8; 136]{
        let mut bytes = [0u8; 136];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        // Check if idx is Some and copy its bytes if it exists
        if let Some(idx) = &self.idx {
            bytes[128..136].copy_from_slice(&idx.to_le_bytes());
        }
        bytes
    }
}

impl TryFrom<&[u8]> for PartialWallet {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<PartialWallet> {
        if bytes.len() != 136 {
            return Err(CompactEcashError::Deserialization(format!(
                "PartialWallet should be exactly 136 bytes, got {}",
                bytes.len()
            )));
        }

        let sig_bytes: &[u8; 96] = &bytes[..96].try_into().expect("Slice size != 96");
        let v_bytes: &[u8; 32] = &bytes[96..128].try_into().expect("Slice size != 32");
        let idx_bytes: &[u8; 8] = &bytes[128..136].try_into().expect("Slice size != 8");

        let sig = Signature::try_from(sig_bytes.as_slice()).unwrap();
        let v = Scalar::from_bytes(&v_bytes).unwrap();
        let idx = None;
        if !idx_bytes.iter().all(|&x| x == 0){
            let idx = Some(u64::from_le_bytes(*idx_bytes));
        }

        Ok(PartialWallet{
            sig,
            v,
            idx,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Wallet {
    sig: Signature,
    v: Scalar,
    pub l: Cell<u64>,
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

    pub fn to_bytes(&self) -> [u8; 136]{
        let mut bytes = [0u8; 136];
        bytes[0..96].copy_from_slice(&self.sig.to_bytes());
        bytes[96..128].copy_from_slice(&self.v.to_bytes());
        bytes[128..136].copy_from_slice(&self.l.get().to_le_bytes());
        bytes
    }
    fn up(&self) {
        self.l.set(self.l.get() + 1);
    }


    pub fn spend(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        bench_flag: bool,
        spend_vv: u64,
    ) -> Result<(Payment, &Self)> {
        if self.l() + spend_vv > params.L() {
            return Err(CompactEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        let grparams = params.grp();
        // randomize signature in the wallet
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grparams);
        // construct kappa i.e., blinded attributes for show
        let attributes = vec![sk_user.sk, self.v()];
        // compute kappa
        let kappa = compute_kappa(
            &grparams,
            &verification_key,
            &attributes,
            sign_blinding_factor,
        );

        // pick random openings o_c
        let o_c = grparams.random_scalar();

        // compute commitments C
        let cc = grparams.gen1() * o_c + grparams.gamma1() * self.v();


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

        for k in 0..spend_vv {
            lk.push(Scalar::from(self.l() + k));

            // compute hashes R_k of the payment info
            let rr_k = hash_to_scalar(pay_info.info);
            rr.push(rr_k);

            let o_a_k = grparams.random_scalar();
            o_a.push(o_a_k);
            let aa_k = grparams.gen1() * o_a_k + grparams.gamma1() * Scalar::from(self.l() + k);
            aa.push(aa_k);

            // evaluate the pseudorandom functions
            let ss_k = pseudorandom_f_delta_v(&grparams, self.v(), self.l() + k);
            ss.push(ss_k);
            let tt_k =
                grparams.gen1() * sk_user.sk + pseudorandom_f_g_v(&grparams, self.v(), self.l() + k) * rr_k;
            tt.push(tt_k);

            // compute values mu, o_mu, lambda, o_lambda
            let mu_k: Scalar = (self.v() + Scalar::from(self.l() + k) + Scalar::from(1))
                .invert()
                .unwrap();
            mu.push(mu_k);

            let o_mu_k = ((o_a_k + o_c) * mu_k).neg();
            o_mu.push(o_mu_k);

            // parse the signature associated with value l+k
            let sign_lk = params.get_sign_by_idx(self.l() + k)?;
            // randomise the signature associated with value l+k
            let (sign_lk_prime, r_k) = sign_lk.randomise(grparams);
            sign_lk_prime_vec.push(sign_lk_prime);
            r_k_vec.push(r_k);
            // compute kappa_k
            let kappa_k = grparams.gen2() * r_k
                + params.pk_rp().alpha
                + params.pk_rp().beta * Scalar::from(self.l() + k);
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
            sig: signature_prime,
            ss: ss.clone(),
            tt: tt.clone(),
            aa: aa.clone(),
            rr: rr.clone(),
            kappa_k: kappa_k_vec.clone(),
            sig_lk: sign_lk_prime_vec,
            cc,
            zk_proof,
            vv: spend_vv,
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
        if bytes.len() != 136 {
            return Err(CompactEcashError::Deserialization(format!(
                "Wallet should be exactly 136 bytes, got {}",
                bytes.len()
            )));
        }

        let sig_bytes: &[u8; 96] = &bytes[..96].try_into().expect("Slice size != 96");
        let v_bytes: &[u8; 32] = &bytes[96..128].try_into().expect("Slice size != 32");
        let l_bytes: &[u8; 8] = &bytes[128..136].try_into().expect("Slice size != 8");

        let sig = Signature::try_from(sig_bytes.as_slice()).unwrap();
        let v = Scalar::from_bytes(&v_bytes).unwrap();
        let l = Cell::new(u64::from_le_bytes(*l_bytes));

        Ok(Wallet{
            sig,
            v,
            l
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
    pub info: [u8; 32],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Payment {
    pub kappa: G2Projective,
    pub sig: Signature,
    pub ss: Vec<G1Projective>,
    pub tt: Vec<G1Projective>,
    pub aa: Vec<G1Projective>,
    pub rr: Vec<Scalar>,
    pub kappa_k: Vec<G2Projective>,
    pub sig_lk: Vec<Signature>,
    pub cc: G1Projective,
    pub zk_proof: SpendProof,
    pub vv: u64,
}

impl Payment {
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo,
    ) -> Result<bool> {
        if bool::from(self.sig.0.is_identity()) {
            return Err(CompactEcashError::Spend(
                "The element h of the signature equals the identity".to_string(),
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

        for k in 0..self.vv {
            if bool::from(self.sig_lk[k as usize].0.is_identity()) {
                return Err(CompactEcashError::Spend(
                    "The element h of the signature on l equals the identity".to_string(),
                ));
            }

            if !check_bilinear_pairing(
                &self.sig_lk[k as usize].0.to_affine(),
                &G2Prepared::from(self.kappa_k[k as usize].to_affine()),
                &self.sig_lk[k as usize].1.to_affine(),
                params.grp().prepared_miller_g2(),
            ) {
                return Err(CompactEcashError::Spend(
                    "The bilinear check for kappa_l failed".to_string(),
                ));
            }
            // verify integrity of R_k
            if !(self.rr[k as usize] == hash_to_scalar(pay_info.info)) {
                return Err(CompactEcashError::Spend(
                    "Integrity of R_k does not hold".to_string(),
                ));
            }
        }

        //TODO: verify whether payinfo contains merchent's identifier

        // verify the zk proof
        let instance = SpendInstance {
            kappa: self.kappa,
            aa: self.aa.clone(),
            cc: self.cc,
            ss: self.ss.clone(),
            tt: self.tt.clone(),
            kappa_k: self.kappa_k.clone(),
        };

        if !self
            .zk_proof
            .verify(&params, &instance, &verification_key, &self.rr)
        {
            return Err(CompactEcashError::Spend(
                "ZkProof verification failed".to_string(),
            ));
        }

        Ok(true)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let kappa_bytes = self.kappa.to_affine().to_compressed();
        let sig_bytes = self.sig.to_bytes();
        let cc_bytes = self.cc.to_affine().to_compressed();
        let vv_bytes: [u8; 8] = self.vv.to_le_bytes();
        let ss_len =  self.ss.len() as u64;
        let tt_len =  self.tt.len() as u64;
        let aa_len =  self.aa.len() as u64;
        let rr_len =  self.rr.len() as u64;
        let kappa_k_len = self.kappa_k.len() as u64;
        let sig_lk_len = self.sig_lk.len() as u64;
        let zk_proof_bytes = self.zk_proof.to_bytes();
        let zk_proof_bytes_len = self.zk_proof.to_bytes().len() as u64;

        let mut bytes: Vec<u8> = Vec::with_capacity(
            (96 + 96 + 48 + 8 + ss_len * 48 + 8 + tt_len * 48 + 8 + aa_len * 48 + 8 + rr_len * 32 + 8 + kappa_k_len * 96 + 8 + sig_lk_len * 96 + zk_proof_bytes_len) as usize);


        bytes.extend_from_slice(&kappa_bytes);
        bytes.extend_from_slice(&sig_bytes);
        bytes.extend_from_slice(&cc_bytes);
        bytes.extend_from_slice(&vv_bytes);


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

        let rr_len_bytes = rr_len.to_le_bytes();
        bytes.extend_from_slice(&rr_len_bytes);
        for r in &self.rr {
            bytes.extend_from_slice(&r.to_bytes());
        }

        let kappa_k_len_bytes = kappa_k_len.to_le_bytes();
        bytes.extend_from_slice(&kappa_k_len_bytes);
        for kk in &self.kappa_k {
            bytes.extend_from_slice(&kk.to_affine().to_compressed());
        }

        let sig_lk_len_bytes = sig_lk_len.to_le_bytes();
        bytes.extend_from_slice(&sig_lk_len_bytes);
        for sig in &self.sig_lk {
            bytes.extend_from_slice(&sig.to_bytes());
        }

        bytes.extend_from_slice(&zk_proof_bytes);
        bytes
    }
}

impl TryFrom<&[u8]> for Payment {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Payment> {
        if bytes.len() < 656 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for Payment deserialization".to_string(),
            ));
        }

        let kappa_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        let sig_bytes: [u8; 96] = bytes[96..192].try_into().unwrap();
        let cc_bytes: [u8; 48] = bytes[192..240].try_into().unwrap();
        let vv_bytes: [u8; 8] = bytes[240..248].try_into().unwrap();
        let ss_len = u64::from_le_bytes(bytes[248..256].try_into().unwrap()) as usize;

        // Convert the byte arrays back into their respective types
        let kappa = try_deserialize_g2_projective(
            &kappa_bytes,
            CompactEcashError::Deserialization("Failed to deserialize kappa".to_string()),
        )?;
        let sig = Signature::try_from(sig_bytes.as_slice())?;

        let cc = try_deserialize_g1_projective(
            &cc_bytes,
            CompactEcashError::Deserialization("Failed to deserialize cc".to_string()),
        )?;
        let vv = u64::from_le_bytes(vv_bytes);

        let mut idx = 256;
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

        let tt_len = u64::from_le_bytes(bytes[idx..idx+8].try_into().unwrap()) as usize;
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

        let aa_len = u64::from_le_bytes(bytes[idx..idx+8].try_into().unwrap()) as usize;
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

        let rr_len = u64::from_le_bytes(bytes[idx..idx+8].try_into().unwrap()) as usize;
        idx += 8;
        let mut rr = Vec::with_capacity(rr_len);
        for _ in 0..rr_len {
            let rr_bytes: [u8; 32] = bytes[idx..idx + 32].try_into().unwrap();
            let rr_elem = try_deserialize_scalar(
                &rr_bytes,
                CompactEcashError::Deserialization("Failed to deserialize rr element".to_string()),
            )?;
            rr.push(rr_elem);
            idx += 32;
        }

        let kappa_k_len = u64::from_le_bytes(bytes[idx..idx+8].try_into().unwrap()) as usize;
        idx += 8;
        let mut kappa_k = Vec::with_capacity(kappa_k_len);
        for _ in 0..kappa_k_len {
            let kappa_k_bytes: [u8; 96] = bytes[idx..idx + 96].try_into().unwrap();
            let kappa_k_elem = try_deserialize_g2_projective(
                &kappa_k_bytes,
                CompactEcashError::Deserialization("Failed to deserialize kappa_k element".to_string()),
            )?;
            kappa_k.push(kappa_k_elem);
            idx += 96;
        }

        // sig_lk
        let sig_lk_len = u64::from_le_bytes(bytes[idx..idx+8].try_into().unwrap()) as usize;
        idx += 8;
        let mut sig_lk = Vec::with_capacity(sig_lk_len);
        for _ in 0..sig_lk_len {
            let sig_lk_bytes: [u8; 96] = bytes[idx..idx + 96].try_into().unwrap();
            let sig_lk_elem = Signature::try_from(sig_lk_bytes.as_slice())?;
            sig_lk.push(sig_lk_elem);
            idx += 96;
        }

        // Deserialize the SpendProof struct
        let zk_proof_bytes = &bytes[idx..];
        let zk_proof = SpendProof::try_from(zk_proof_bytes)?;

        // Construct the Payment struct from the deserialized data
        let payment = Payment {
            kappa,
            sig,
            ss,
            tt,
            aa,
            rr,
            kappa_k,
            sig_lk,
            cc,
            zk_proof,
            vv,
        };

        Ok(payment)
    }
}