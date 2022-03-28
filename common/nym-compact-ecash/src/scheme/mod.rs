use std::cell::Cell;
use std::convert::TryFrom;
use std::convert::TryInto;

use bls12_381::{G1Projective, G2Prepared, G2Projective, Scalar};
use group::Curve;

use crate::Attribute;
use crate::error::{CompactEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::Parameters;
use crate::traits::Bytable;
use crate::utils::{check_bilinear_pairing, hash_to_scalar, try_deserialize_g1_projective};

pub mod aggregation;
pub mod keygen;
pub mod setup;
pub mod spend;
pub mod withdrawal;
pub mod identify;

pub type SignerIndex = u64;

#[derive(Debug, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Signature(pub(crate) G1Projective, pub(crate) G1Projective);

pub type PartialSignature = Signature;

impl TryFrom<&[u8]> for Signature {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Signature> {
        if bytes.len() != 96 {
            return Err(CompactEcashError::Deserialization(format!(
                "Signature must be exactly 96 bytes, got {}",
                bytes.len()
            )));
        }

        let sig1_bytes: &[u8; 48] = &bytes[..48].try_into().expect("Slice size != 48");
        let sig2_bytes: &[u8; 48] = &bytes[48..].try_into().expect("Slice size != 48");

        let sig1 = try_deserialize_g1_projective(
            sig1_bytes,
            CompactEcashError::Deserialization("Failed to deserialize compressed sig1".to_string()),
        )?;

        let sig2 = try_deserialize_g1_projective(
            sig2_bytes,
            CompactEcashError::Deserialization("Failed to deserialize compressed sig2".to_string()),
        )?;

        Ok(Signature(sig1, sig2))
    }
}

impl Signature {
    pub(crate) fn sig1(&self) -> &G1Projective {
        &self.0
    }

    pub(crate) fn sig2(&self) -> &G1Projective {
        &self.1
    }

    pub fn randomise(&self, params: &Parameters) -> (Signature, Scalar) {
        let r = params.random_scalar();
        let r_prime = params.random_scalar();
        let h_prime = self.0 * r_prime;
        let s_prime = (self.1 * r_prime) + (h_prime * r);
        (Signature(h_prime, s_prime), r)
    }

    pub fn to_bytes(self) -> [u8; 96] {
        let mut bytes = [0u8; 96];
        bytes[..48].copy_from_slice(&self.0.to_affine().to_compressed());
        bytes[48..].copy_from_slice(&self.1.to_affine().to_compressed());
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Signature> {
        Signature::try_from(bytes)
    }
}

impl Bytable for Signature {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self> {
        Signature::from_bytes(slice)
    }
}


#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub struct BlindedSignature(G1Projective, G1Projective);

pub struct SignatureShare {
    signature: Signature,
    index: SignerIndex,
}

impl SignatureShare {
    pub fn new(signature: Signature, index: SignerIndex) -> Self {
        SignatureShare { signature, index }
    }

    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn index(&self) -> SignerIndex {
        self.index
    }

    // pub fn aggregate(shares: &[Self]) -> Result<Signature> {
    //     aggregate_signature_shares(shares)
    // }
}

pub struct PartialWallet {
    sig: Signature,
    v: Scalar,
    idx: Option<SignerIndex>,
}

impl PartialWallet {
    pub fn signature(&self) -> &Signature { &self.sig }
    pub fn v(&self) -> Scalar { self.v }
    pub fn index(&self) -> Option<SignerIndex> {
        self.idx
    }
}

pub struct Wallet {
    sig: Signature,
    v: Scalar,
    t: Scalar,
    l: Cell<u64>,
}

impl Wallet {
    pub fn signature(&self) -> &Signature { &self.sig }
    pub fn v(&self) -> Scalar { self.v }
    pub fn t(&self) -> Scalar { self.t }
    pub fn l(&self) -> u64 { self.l.get() }

    fn up(&self) {
        self.l.set(self.l.get() + 1);
    }

    pub fn spend(&self, params: &Parameters, verification_key: &VerificationKeyAuth, skUser: &SecretKeyUser, payInfo: &PayInfo) -> Result<(Payment, &Self)> {
        if self.l() > params.L() {
            return Err(CompactEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        // randomize signature in the wallet
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(params);
        // construct kappa i.e., blinded attributes for show
        let attributes = vec![skUser.sk, self.v(), self.t()];
        let kappa = compute_kappa(&params, &verification_key, &attributes, sign_blinding_factor);

        // pick random openings o_a, o_c, o_d
        let o_a = params.random_scalar();
        let o_c = params.random_scalar();
        let o_d = params.random_scalar();

        // compute commitments A, C, D
        let A = params.gen1() * o_a + params.gamma1().unwrap() * Scalar::from(self.l());
        let C = params.gen1() * o_c + params.gamma1().unwrap() * self.v();
        let D = params.gen1() * o_d + params.gamma1().unwrap() * self.t();

        // compute hash of the payment info
        let R = hash_to_scalar(payInfo.info);

        // evaluate the pseudorandom functions
        let S = pseudorandom_fgv(&params, self.v(), self.l());
        let T = params.gen1() * skUser.sk + pseudorandom_fgt(&params, self.t(), self.l()) * R;

        // compute values mu, o_mu, lambda, o_lambda
        let mu: Scalar = (self.v() + Scalar::from(self.l()) + Scalar::from(1)).invert().unwrap();
        let o_mu = ((o_a + o_c) * mu).neg();
        let lambda = (self.t() + Scalar::from(self.l()) + Scalar::from(1)).invert().unwrap();
        let o_lambda = ((o_a + o_d) * lambda).neg();

        // construct the zkp proof
        let spendInstance = SpendInstance {
            kappa,
            A,
            C,
            D,
            S,
            T,
        };
        let spendWitness = SpendWitness {
            attributes,
            r: sign_blinding_factor,
            l: Scalar::from(self.l()),
            o_a,
            o_c,
            o_d,
            mu,
            lambda,
            o_mu,
            o_lambda,
        };
        let zk_proof = SpendProof::construct(&params, &spendInstance, &spendWitness, &verification_key, R);

        // output pay and updated wallet
        let pay = Payment {
            kappa,
            sig: signature_prime,
            S,
            T,
            A,
            C,
            D,
            R,
            zk_proof,
        };

        self.up();

        Ok((pay, self))
    }
}

pub fn pseudorandom_fgv(params: &Parameters, v: Scalar, l: u64) -> G1Projective {
    let pow = (v + Scalar::from(l) + Scalar::from(1)).neg();
    params.gen1() * pow
}

pub fn pseudorandom_fgt(params: &Parameters, t: Scalar, l: u64) -> G1Projective {
    let pow = (t + Scalar::from(l) + Scalar::from(1)).neg();
    params.gen1() * pow
}

pub fn compute_kappa(
    params: &Parameters,
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

pub struct PayInfo {
    pub(crate) info: [u8; 32],
}

pub struct Payment {
    pub kappa: G2Projective,
    pub sig: Signature,
    pub S: G1Projective,
    pub T: G1Projective,
    pub A: G1Projective,
    pub C: G1Projective,
    pub D: G1Projective,
    pub R: Scalar,
    pub zk_proof: SpendProof,
}

impl Payment {
    pub fn spend_verify(&self, params: &Parameters, verification_key: &VerificationKeyAuth, payinfo: &PayInfo) -> Result<bool> {
        if bool::from(self.sig.0.is_identity()) {
            return Err(CompactEcashError::Spend(
                "The element h of the signature equals the identity".to_string(),
            ));
        }

        if !check_bilinear_pairing(
            &self.sig.0.to_affine(),
            &G2Prepared::from(self.kappa.to_affine()),
            &self.sig.1.to_affine(),
            params.prepared_miller_g2(),
        ) {
            return Err(CompactEcashError::Spend(
                "The bilinear check for kappa failed".to_string(),
            ));
        }

        // verify integrity of R
        if !(self.R == hash_to_scalar(payinfo.info)) {
            return Err(CompactEcashError::Spend(
                "Integrity of R does not hold".to_string(),
            ));
        }

        //TODO: verify whether payinfo contains merchent's identifier

        // verify the zk proof
        let instance = SpendInstance {
            kappa: self.kappa,
            A: self.A,
            C: self.C,
            D: self.D,
            S: self.S,
            T: self.T,
        };

        if !self.zk_proof.verify(&params, &instance, &verification_key, self.R) {
            return Err(CompactEcashError::Spend(
                "ZkProof verification failed".to_string(),
            ));
        }

        Ok(true)
    }
}