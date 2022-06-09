use std::convert::TryFrom;
use std::fmt::Debug;
use std::ops::Neg;

use bls12_381::{G1Projective, G2Projective, Gt, pairing, Scalar};
use group::Curve;

use crate::Attribute;
use crate::scheme::setup::GroupParameters;

#[derive(Debug, Clone)]
pub struct SPSVerificationKey {
    pub grp: GroupParameters,
    pub uus: Vec<G1Projective>,
    pub wws: Vec<G2Projective>,
    pub yy: G2Projective,
    pub zz: G2Projective,
}

pub struct SPSSecretKey {
    sps_vk: SPSVerificationKey,
    us: Vec<Scalar>,
    ws: Vec<Scalar>,
    y: Scalar,
    z: Scalar,
}

impl SPSSecretKey {
    pub fn z(&self) -> Scalar {
        self.z
    }

    pub fn y(&self) -> Scalar {
        self.y
    }

    pub fn sign(&self, grp: &GroupParameters, messages_a: Option<&[G1Projective]>, messages_b: Option<&[G2Projective]>) -> SPSSignature {
        let r = grp.random_scalar();
        let rr = grp.gen1() * r;
        let ss: G1Projective = match messages_a {
            Some(msgs_a) => {
                let prod_s: Vec<G1Projective> = msgs_a
                    .iter()
                    .zip(self.ws.iter())
                    .map(|(m_i, w_i)| m_i * w_i.neg())
                    .collect();
                grp.gen1() * (self.z() - r * self.y()) + prod_s.iter().fold(G1Projective::identity(), |acc, elem| acc + elem)
            }
            None => grp.gen1() * (self.z() - r * self.y())
        };
        let tt = match messages_b {
            Some(msgs_b) => {
                let prod_t: Vec<G2Projective> = msgs_b
                    .iter()
                    .zip(self.us.iter())
                    .map(|(m_i, u_i)| m_i * u_i.neg())
                    .collect();
                (grp.gen2() + prod_t.iter().fold(G2Projective::identity(), |acc, elem| acc + elem)) * r.invert().unwrap()
            }
            None => grp.gen2() * r.invert().unwrap()
        };

        SPSSignature
        {
            rr,
            ss,
            tt,
        }
    }
}

impl SPSVerificationKey {
    pub fn verify(&self, grp: &GroupParameters, signature: SPSSignature, messages_a: &[G1Projective], messages_b: Option<&[G2Projective]>) -> bool {
        let pg_rr_yy = pairing(&signature.rr.to_affine(), &self.yy.to_affine());
        let pg_ss_g2 = pairing(&signature.ss.to_affine(), grp.gen2());
        let pg_g1_zz = pairing(grp.gen1(), &self.zz.to_affine());
        let pg_ma_ww: Vec<Gt> = messages_a.iter()
            .zip(self.wws.iter())
            .map(|(ma, ww)| pairing(&ma.to_affine(), &ww.to_affine()))
            .collect();

        let mut prod_pg_ma_ww = Gt::identity();
        for elem in pg_ma_ww.iter() {
            prod_pg_ma_ww = prod_pg_ma_ww + elem;
        }

        // let prod_pg_ma_ww = pg_ma_ww.iter().fold(Gt::identity() | acc, elem | acc + elem);

        assert_eq!(pg_rr_yy + pg_ss_g2 + prod_pg_ma_ww, pg_g1_zz);

        let result = match messages_b {
            Some(msgs_b) => {
                let pg_rr_tt = pairing(&signature.rr.to_affine(), &signature.tt.to_affine());
                let pg_g1_g2 = pairing(grp.gen1(), grp.gen2());
                let pg_uu_mb: Vec<Gt> = self.uus.iter()
                    .zip(msgs_b.iter())
                    .map(|(uu, mb)| pairing(&uu.to_affine(), &mb.to_affine()))
                    .collect();

                let mut prod_pg_uu_mb = Gt::identity();
                for elem in pg_uu_mb.iter() {
                    prod_pg_uu_mb = prod_pg_uu_mb + elem;
                }
                // let prod_pg_uu_mb = pg_uu_mb.iter().fold(Gt::identity() | acc, elem | acc + elem);
                if pg_rr_tt + prod_pg_uu_mb == pg_g1_g2 {
                    true
                } else {
                    false
                }
            }

            None => {
                let pg_sign_rr_yy = pairing(&signature.rr.to_affine(), &self.yy.to_affine());
                let pg_sign_ss_gen2 = pairing(&signature.ss.to_affine(), &grp.gen2());
                let pg_ma_wws: Vec<Gt> = messages_a.iter()
                    .zip(self.wws.iter())
                    .map(|(ma, ww)| pairing(&ma.to_affine(), &ww.to_affine()))
                    .collect();

                let mut prod_pg_ma_wws = Gt::identity();
                for elem in pg_ma_wws.iter() {
                    prod_pg_ma_wws = prod_pg_ma_wws + elem;
                }

                let pg_gen1_zz = pairing(&grp.gen1(), &self.zz.to_affine());

                let pg_rr_tt = pairing(&signature.rr.to_affine(), &signature.tt.to_affine());
                let pg_gen1_gen2 = pairing(&grp.gen1(), &grp.gen2());

                assert_eq!(pg_sign_rr_yy + pg_sign_ss_gen2 + prod_pg_ma_wws, pg_gen1_zz);
                assert_eq!(pg_rr_tt, pg_gen1_gen2);

                if pg_sign_rr_yy + pg_sign_ss_gen2 + prod_pg_ma_wws == pg_gen1_zz && pg_rr_tt == pg_gen1_gen2 {
                    true
                } else {
                    false
                }
            }
        };

        return result;
    }

    pub fn get_ith_ww(&self, idx: usize) -> &G2Projective { return self.wws.get(idx).unwrap(); }

    pub fn get_zz(&self) -> &G2Projective { return &self.zz; }

    pub fn get_yy(&self) -> &G2Projective { return &self.yy; }
}

pub struct SPSKeyPair {
    pub sps_sk: SPSSecretKey,
    pub sps_vk: SPSVerificationKey,
}

impl SPSKeyPair {
    pub fn new(grp: GroupParameters, a: usize, b: usize) -> SPSKeyPair {
        let us = grp.n_random_scalars(b);
        let ws = grp.n_random_scalars(a);
        let y = grp.random_scalar();
        let z = grp.random_scalar();
        let uus: Vec<G1Projective> = us.iter().map(|u| grp.gen1() * u).collect();
        let yy = grp.gen2() * y;
        let wws: Vec<G2Projective> = ws.iter().map(|w| grp.gen2() * w).collect();
        let zz = grp.gen2() * z;

        let sps_vk = SPSVerificationKey {
            grp: grp.clone(),
            uus,
            wws,
            yy,
            zz,
        };
        let sps_sk = SPSSecretKey {
            sps_vk: sps_vk.clone(),
            us,
            ws,
            y,
            z,
        };
        SPSKeyPair { sps_sk, sps_vk }
    }
}

#[derive(Debug, Clone)]
pub struct SPSSignature {
    pub rr: G1Projective,
    pub ss: G1Projective,
    pub tt: G2Projective,
}


#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use crate::scheme::setup::GroupParameters;
    use crate::scheme::structure_preserving_signature::SPSKeyPair;
    use crate::utils::{hash_g1, hash_g2};

    #[test]
    fn sign_and_verify_for_two_msg_in_G1_and_two_msgs_in_G2() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let sps_keypair = SPSKeyPair::new(grp.clone(), 2, 2);
        let msgs_a = vec![hash_g1("messageA1"), hash_g1("messageA2")];
        let msgs_b = vec![hash_g2("messageB1"), hash_g2("messageB2")];
        let signature = sps_keypair.sps_sk.sign(&grp, Some(&msgs_a), Some(&msgs_b));
        assert!(sps_keypair.sps_vk.verify(&grp, signature, &msgs_a, Some(&msgs_b)));
    }

    #[test]
    fn sign_and_verify_for_two_msg_in_G1_and_no_msgs_in_G2() {
        let rng = thread_rng();
        let grp = GroupParameters::new().unwrap();
        let sps_keypair = SPSKeyPair::new(grp.clone(), 2, 2);
        let msgs_a = vec![hash_g1("messageA1"), hash_g1("messageA2")];
        let signature = sps_keypair.sps_sk.sign(&grp, Some(&msgs_a), None);
        assert!(sps_keypair.sps_vk.verify(&grp, signature, &msgs_a, None));
    }
}