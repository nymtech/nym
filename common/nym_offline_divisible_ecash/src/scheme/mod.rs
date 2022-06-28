use std::cell::Cell;
use std::convert::{TryFrom, TryInto};
use std::ops::Neg;

use bls12_381::{G1Projective, G2Prepared, G2Projective, pairing, Scalar};
use group::{Curve, GroupEncoding};

use crate::Attribute;
use crate::constants::L;
use crate::error::{DivisibleEcashError, Result};
use crate::proofs::proof_spend::{SpendInstance, SpendProof, SpendWitness};
use crate::scheme::keygen::{SecretKeyUser, VerificationKeyAuth};
use crate::scheme::setup::{GroupParameters, Parameters};
use crate::utils::{check_bilinear_pairing, hash_to_scalar, Signature, SignerIndex, try_deserialize_g1_projective};

pub mod aggregation;
pub mod keygen;
pub mod setup;
pub mod structure_preserving_signature;
pub mod withdrawal;
pub mod identification;

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

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct Phi(pub(crate) G1Projective, pub(crate) G1Projective);

impl Phi {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(48 + 48);
        bytes.extend_from_slice(self.0.to_bytes().as_ref());
        bytes.extend_from_slice(self.1.to_bytes().as_ref());
        bytes
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 48 * 2 || (bytes.len()) % 48 != 0 {
            return Err(DivisibleEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len(),
                target: 48 * 2,
                modulus: 48,
                object: "phi".to_string(),
            });
        }

        let elem_0_bytes = bytes[0..48].try_into().unwrap();
        let elem_0 = try_deserialize_g1_projective(
            elem_0_bytes,
            DivisibleEcashError::Deserialization("Failed to deserialize element 0 of Phi".to_string()),
        )?;

        let elem_1_bytes = bytes[48..96].try_into().unwrap();
        let elem_1 = try_deserialize_g1_projective(
            elem_1_bytes,
            DivisibleEcashError::Deserialization("Failed to deserialize element 1 of Phi".to_string()),
        )?;

        Ok(Phi(elem_0, elem_1))
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy)]
pub struct VarPhi(pub(crate) G1Projective, pub(crate) G1Projective);

impl VarPhi {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(48 + 48);
        bytes.extend_from_slice(self.0.to_bytes().as_ref());
        bytes.extend_from_slice(self.1.to_bytes().as_ref());
        bytes
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 48 * 2 || (bytes.len()) % 48 != 0 {
            return Err(DivisibleEcashError::DeserializationInvalidLength {
                actual: bytes.len(),
                modulus_target: bytes.len(),
                target: 48 * 2,
                modulus: 48,
                object: "varphi".to_string(),
            });
        }

        let elem_0_bytes = bytes[0..48].try_into().unwrap();
        let elem_0 = try_deserialize_g1_projective(
            elem_0_bytes,
            DivisibleEcashError::Deserialization("Failed to deserialize element 0 of VarPhi".to_string()),
        )?;

        let elem_1_bytes = bytes[48..96].try_into().unwrap();
        let elem_1 = try_deserialize_g1_projective(
            elem_1_bytes,
            DivisibleEcashError::Deserialization("Failed to deserialize element 1 of VarPhi".to_string()),
        )?;

        Ok(VarPhi(elem_0, elem_1))
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PayInfo {
    pub info: [u8; 32],
}

#[derive(Debug, Clone)]
pub struct Payment {
    pub kappa: G2Projective,
    pub sig: Signature,
    pub phi: Phi,
    pub varphi: VarPhi,
    pub varsig_prime1: G1Projective,
    pub varsig_prime2: G1Projective,
    pub theta_prime1: G1Projective,
    pub theta_prime2: G1Projective,
    pub rr_prime: G1Projective,
    pub ss_prime: G1Projective,
    pub tt_prime: G2Projective,
    pub rr: Scalar,
    pub zk_proof: SpendProof,
    pub vv: u64,
}

impl Payment {
    pub fn get_kappa(&self) -> G2Projective { self.kappa }
    pub fn get_sig(&self) -> Signature { self.sig }
    pub fn get_phi(&self) -> Phi { self.phi }
    pub fn get_varphi(&self) -> VarPhi { self.varphi }
    pub fn get_varsig_prime1(&self) -> G1Projective { self.varsig_prime1 }
    pub fn get_varsig_prime2(&self) -> G1Projective { self.varsig_prime2 }
    pub fn get_theta_prime1(&self) -> G1Projective { self.theta_prime1 }
    pub fn get_theta_prime2(&self) -> G1Projective { self.theta_prime2 }
    pub fn get_rr_prime(&self) -> G1Projective { self.rr_prime }
    pub fn get_ss_prime(&self) -> G1Projective { self.ss_prime }
    pub fn get_tt_prime(&self) -> G2Projective { self.tt_prime }
    pub fn get_rr(&self) -> Scalar { self.rr }
    pub fn get_zk_proof(&self) -> SpendProof { self.zk_proof.clone() }
    pub fn get_vv(&self) -> u64 { self.vv }
}

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
}

pub struct Wallet {
    sig: Signature,
    v: Scalar,
    l: Cell<u64>,
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


    pub(crate) fn spend(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        sk_user: &SecretKeyUser,
        pay_info: &PayInfo,
        vv: u64,
    ) -> Result<(Payment, &Self)> {
        if self.l() + vv >= L {
            return Err(DivisibleEcashError::Spend(
                "The counter l is higher than max L".to_string(),
            ));
        }

        let grp = params.get_grp();
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();
        // randomize signature in the wallet
        let (signature_prime, sign_blinding_factor) = self.signature().randomise(grp);
        // construct kappa i.e., blinded attributes for show
        let attributes = vec![sk_user.sk, self.v()];
        // compute kappa
        let kappa = compute_kappa(
            &grp,
            &verification_key,
            &attributes,
            sign_blinding_factor,
        );

        let r1 = grp.random_scalar();
        let r2 = grp.random_scalar();
        let phi = Phi(grp.gen1() * r1, params_u.get_ith_sigma(self.l() as usize) * self.v + params_u.get_ith_eta(vv as usize) * r1);

        // compute hash of the payment info
        let rr = hash_to_scalar(pay_info.info);
        let varphi = VarPhi(grp.gen1() * r2, (grp.gen1() * rr) * sk_user.sk + params_u.get_ith_theta(self.l() as usize) * self.v + params_u.get_ith_eta(vv as usize) * r2);


        // random value used to compute blinded bases
        let r_varsig1 = grp.random_scalar();
        let r_theta1 = grp.random_scalar();
        let r_varsig2 = grp.random_scalar();
        let r_theta2 = grp.random_scalar();
        let r_rr = grp.random_scalar();
        let r_ss = grp.random_scalar();
        let r_tt = grp.random_scalar();

        // compute blinded bases
        let psi_g1 = params_u.get_psi_g1();
        let psi_g2 = params_u.get_psi_g2();
        let varsig_prime1 = params_u.get_ith_sigma(self.l() as usize) + (psi_g1 * r_varsig1);
        let theta_prime1 = params_u.get_ith_theta(self.l() as usize) + (psi_g1 * r_theta1);
        let varsig_prime2 = params_u.get_ith_sigma(self.l() as usize + vv as usize - 1) + (psi_g1 * r_varsig2);
        let theta_prime2 = params_u.get_ith_theta(self.l() as usize + vv as usize - 1) + (psi_g1 * r_theta2);

        let tau_l_vv = params_u.get_ith_sps_sign(self.l() as usize + vv as usize - 1);
        let rr_prime = tau_l_vv.rr + (psi_g1 * r_rr);
        let ss_prime = tau_l_vv.ss + (psi_g1 * r_ss);
        let tt_prime = tau_l_vv.tt + (psi_g2 * r_tt);

        let rho1 = self.v.neg() * r_varsig1;
        let rho2 = self.v.neg() * r_theta1;
        let rho3 = r_rr * r_tt;

        let pg_varsigpr1_delta = pairing(&varsig_prime1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_psi0_delta = pairing(&psi_g1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_varsigpr2_gen2 = pairing(&varsig_prime2.to_affine(), grp.gen2());
        let pg_psi0_gen2 = pairing(&psi_g1.to_affine(), grp.gen2());
        let pg_thetapr1_delta = pairing(&theta_prime1.to_affine(), &params_a.get_ith_delta((vv - 1) as usize).to_affine());
        let pg_thetapr2_gen2 = pairing(&theta_prime2.to_affine(), grp.gen2());
        let yy = params_u.get_sps_pk().get_yy();
        let pg_rrprime_yy = pairing(&rr_prime.to_affine(), &yy.to_affine());
        let pg_psi0_yy = pairing(&psi_g1.to_affine(), &yy.to_affine());
        let pg_ssprime_gen2 = pairing(&ss_prime.to_affine(), grp.gen2());
        let ww1 = params_u.get_sps_pk().get_ith_ww(0);
        let ww2 = params_u.get_sps_pk().get_ith_ww(1);
        let pg_varsigpr2_ww1 = pairing(&varsig_prime2.to_affine(), &ww1.to_affine());
        let pg_psi0_ww1 = pairing(&psi_g1.to_affine(), &ww1.to_affine());
        let pg_thetapr2_ww2 = pairing(&theta_prime2.to_affine(), &ww2.to_affine());
        let pg_psi0_ww2 = pairing(&psi_g1.to_affine(), &ww2.to_affine());
        let pg_gen1_zz = pairing(grp.gen1(), &params_u.get_sps_pk().get_zz().to_affine());
        let pg_rr_tt = pairing(&rr_prime.to_affine(), &tt_prime.to_affine());
        let pg_rr_psi1 = pairing(&rr_prime.to_affine(), &psi_g2.to_affine());
        let pg_psi0_tt = pairing(&psi_g1.to_affine(), &tt_prime.to_affine());
        let pg_psi0_psi1 = pairing(&psi_g1.to_affine(), &psi_g2.to_affine());
        let pg_gen1_gen2 = pairing(grp.gen1(), grp.gen2());

        let pg_eq1 = pg_varsigpr1_delta - pg_varsigpr2_gen2;
        let pg_eq2 = pg_thetapr1_delta - pg_thetapr2_gen2;
        let pg_eq3 = pg_rrprime_yy + pg_ssprime_gen2 + pg_varsigpr2_ww1 + pg_thetapr2_ww2 + pg_gen1_zz.neg();
        let pg_eq4 = pg_rr_tt - pg_gen1_gen2;

        let instance = SpendInstance {
            kappa,
            phi,
            varphi,
            rr,
            rr_prime,
            ss_prime,
            tt_prime,
            varsig_prime1,
            theta_prime1,
            pg_eq1,
            pg_eq2,
            pg_eq3,
            pg_eq4,
            psi_g1: *psi_g1,
            psi_g2: *psi_g2,
            pg_psi0_delta,
            pg_psi0_gen2,
            pg_psi0_yy,
            pg_psi0_ww1,
            pg_psi0_ww2,
            pg_rr_psi1,
            pg_psi0_tt,
            pg_psi0_psi1,
        };

        let witness = SpendWitness {
            sk_u: sk_user.clone(),
            v: self.v,
            r: sign_blinding_factor,
            r1,
            r2,
            r_varsig1,
            r_theta1,
            r_varsig2,
            r_theta2,
            r_rr,
            r_ss,
            r_tt,
            rho1,
            rho2,
            rho3,
        };

        // compute the zk proof
        let zk_proof = SpendProof::construct(params, &instance, &witness, &verification_key, vv);

        // output pay and updated wallet
        let pay = Payment {
            kappa,
            sig: signature_prime,
            phi,
            varphi,
            varsig_prime1,
            varsig_prime2,
            theta_prime1,
            theta_prime2,
            rr_prime,
            ss_prime,
            tt_prime,
            rr,
            zk_proof,
            vv,
        };
        
        self.l.set(self.l() + vv);
        Ok((pay, self))
    }
}

impl Payment {
    pub fn spend_verify(
        &self,
        params: &Parameters,
        verification_key: &VerificationKeyAuth,
        pay_info: &PayInfo) -> Result<bool> {
        if bool::from(self.sig.0.is_identity()) {
            return Err(DivisibleEcashError::Spend(
                "The element h of the signature equals the identity".to_string(),
            ));
        }
        let grp = params.get_grp();
        let params_a = params.get_params_a();
        let params_u = params.get_params_u();

        if !check_bilinear_pairing(
            &self.sig.0.to_affine(),
            &G2Prepared::from(self.kappa.to_affine()),
            &self.sig.1.to_affine(),
            grp.prepared_miller_g2(),
        ) {
            return Err(DivisibleEcashError::Spend(
                "The bilinear check for kappa failed".to_string(),
            ));
        }

        if bool::from(self.sig.0.is_identity()) {
            return Err(DivisibleEcashError::Spend(
                "The element h of the signature on l equals the identity".to_string(),
            ));
        }

        // verify integrity of R
        if !(self.rr == hash_to_scalar(pay_info.info)) {
            return Err(DivisibleEcashError::Spend(
                "Integrity of R does not hold".to_string(),
            ));
        }

        //TODO: verify whether payinfo contains merchent's identifier

        let psi_g1 = params_u.get_psi_g1();
        let psi_g2 = params_u.get_psi_g2();
        let pg_varsigpr1_delta = pairing(&self.varsig_prime1.to_affine(), &params_a.get_ith_delta((self.vv - 1) as usize).to_affine());
        let pg_psi0_delta = pairing(&psi_g1.to_affine(), &params_a.get_ith_delta((self.vv - 1) as usize).to_affine());
        let pg_varsigpr2_gen2 = pairing(&self.varsig_prime2.to_affine(), grp.gen2());
        let pg_psi0_gen2 = pairing(&psi_g1.to_affine(), grp.gen2());
        let pg_thetapr1_delta = pairing(&self.theta_prime1.to_affine(), &params_a.get_ith_delta((self.vv - 1) as usize).to_affine());
        let pg_thetapr2_gen2 = pairing(&self.theta_prime2.to_affine(), grp.gen2());
        let yy = params_u.get_sps_pk().get_yy();
        let pg_rrprime_yy = pairing(&self.rr_prime.to_affine(), &yy.to_affine());
        let pg_psi0_yy = pairing(&psi_g1.to_affine(), &yy.to_affine());
        let pg_ssprime_gen2 = pairing(&self.ss_prime.to_affine(), grp.gen2());
        let ww1 = params_u.get_sps_pk().get_ith_ww(0);
        let ww2 = params_u.get_sps_pk().get_ith_ww(1);
        let pg_varsigpr2_ww1 = pairing(&self.varsig_prime2.to_affine(), &ww1.to_affine());
        let pg_psi0_ww1 = pairing(&psi_g1.to_affine(), &ww1.to_affine());
        let pg_thetapr2_ww2 = pairing(&self.theta_prime2.to_affine(), &ww2.to_affine());
        let pg_psi0_ww2 = pairing(&psi_g1.to_affine(), &ww2.to_affine());
        let pg_gen1_zz = pairing(grp.gen1(), &params_u.get_sps_pk().get_zz().to_affine());
        let pg_rr_tt = pairing(&self.rr_prime.to_affine(), &self.tt_prime.to_affine());
        let pg_rr_psi1 = pairing(&self.rr_prime.to_affine(), &psi_g2.to_affine());
        let pg_psi0_tt = pairing(&psi_g1.to_affine(), &self.tt_prime.to_affine());
        let pg_psi0_psi1 = pairing(&psi_g1.to_affine(), &psi_g2.to_affine());
        let pg_gen1_gen2 = pairing(grp.gen1(), grp.gen2());

        let pg_eq1 = pg_varsigpr1_delta - pg_varsigpr2_gen2;
        let pg_eq2 = pg_thetapr1_delta - pg_thetapr2_gen2;
        let pg_eq3 = pg_rrprime_yy + pg_ssprime_gen2 + pg_varsigpr2_ww1 + pg_thetapr2_ww2 + pg_gen1_zz.neg();
        let pg_eq4 = pg_rr_tt - pg_gen1_gen2;

        let instance = SpendInstance {
            kappa: self.kappa,
            phi: self.phi,
            varphi: self.varphi,
            rr: self.rr,
            rr_prime: self.rr_prime,
            ss_prime: self.ss_prime,
            tt_prime: self.tt_prime,
            varsig_prime1: self.varsig_prime1,
            theta_prime1: self.theta_prime1,
            pg_eq1,
            pg_eq2,
            pg_eq3,
            pg_eq4,
            psi_g1: *psi_g1,
            psi_g2: *psi_g2,
            pg_psi0_delta,
            pg_psi0_gen2,
            pg_psi0_yy,
            pg_psi0_ww1,
            pg_psi0_ww2,
            pg_rr_psi1,
            pg_psi0_tt,
            pg_psi0_psi1,
        };

        Ok(self.zk_proof.verify(&params, &instance, &verification_key, self.vv))
    }
}

#[cfg(test)]
mod tests {
    use std::convert::TryFrom;

    use rand::thread_rng;

    use crate::scheme::{PayInfo, Phi, VarPhi, Wallet};
    use crate::scheme::aggregation::aggregate_verification_keys;
    use crate::scheme::keygen::{PublicKeyUser, ttp_keygen_authorities, VerificationKeyAuth};
    use crate::scheme::setup::{GroupParameters, Parameters};
    use crate::utils::hash_g1;

    #[test]
    fn phi_to_and_from_bytes() {
        let phi = Phi(hash_g1("Element 0 of Phi"), hash_g1("Element 1 of Phi"));
        let phi_bytes = phi.to_bytes();
        let phi_from_bytes = Phi::from_bytes(&phi_bytes).unwrap();
        assert_eq!(phi, phi_from_bytes);
    }

    #[test]
    fn varphi_to_and_from_bytes() {
        let varphi = VarPhi(hash_g1("Element 0 of VarPhi"), hash_g1("Element 1 of VarPhi"));
        let varphi_bytes = varphi.to_bytes();
        let varphi_from_bytes = VarPhi::from_bytes(&varphi_bytes).unwrap();
        assert_eq!(varphi, varphi_from_bytes);
    }

    #[test]
    fn spend_verification_is_correct() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let params = Parameters::new(grp.clone());
        let params_u = params.get_params_u();
        let params_a = params.get_params_a();

        let sk = grp.random_scalar();
        let pk_user = PublicKeyUser {
            pk: grp.gen1() * sk,
        };

        let authorities_keypairs = ttp_keygen_authorities(&params, 2, 3).unwrap();
        let verification_keys_auth: Vec<VerificationKeyAuth> = authorities_keypairs
            .iter()
            .map(|keypair| keypair.verification_key())
            .collect();

        let verification_key =
            aggregate_verification_keys(&verification_keys_auth, Some(&[1, 2, 3])).unwrap();
    }
}