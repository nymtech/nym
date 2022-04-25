use std::convert::TryFrom;
use std::ops::Neg;

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::Curve;

use crate::Attribute;
use crate::scheme::setup::GroupParameters;

#[derive(Debug, Clone)]
pub struct SPSVerificationKey {
    pub grparams: GroupParameters,
    pub uus: Vec<G1Projective>,
    pub wws: Vec<G2Projective>,
    pub yy: G2Projective,
    pub zz: G2Projective,
}

pub struct SPSSecretKey {
    spsVK: SPSVerificationKey,
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

    pub fn sign(&self, grparams: GroupParameters, messagesA: Option<&[G1Projective]>, messagesB: Option<&[G2Projective]>) -> SPSSignature {
        let r = grparams.random_scalar();
        let rr = grparams.gen1() * r;
        let ss: G1Projective = match messagesA {
            Some(msgsA) => {
                let prodS: Vec<G1Projective> = msgsA
                    .iter()
                    .zip(self.ws.iter())
                    .map(|(m_i, w_i)| m_i * w_i.neg())
                    .collect();
                grparams.gen1() * (self.z() - r * self.y()) + prodS.iter().fold(G1Projective::identity(), |acc, elem| acc + elem)
            }
            None => grparams.gen1() * (self.z() - r * self.y())
        };
        let tt = match messagesB {
            Some(msgsB) => {
                let prodT: Vec<G2Projective> = msgsB
                    .iter()
                    .zip(self.us.iter())
                    .map(|(m_i, u_i)| m_i * u_i.neg())
                    .collect();
                (grparams.gen2() + prodT.iter().fold(G2Projective::identity(), |acc, elem| acc + elem)) * r.invert().unwrap()
            }
            None => grparams.gen2() * r.invert().unwrap()
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
    pub fn verify() -> bool {
        return true;
    }
}

pub struct SPSKeyPair {
    pub sps_sk: SPSSecretKey,
    pub sps_vk: SPSVerificationKey,
}

impl SPSKeyPair {
    pub fn new(grparams: GroupParameters, a: usize, b: usize) -> SPSKeyPair {
        let us = grparams.n_random_scalars(b);
        let ws = grparams.n_random_scalars(a);
        let y = grparams.random_scalar();
        let z = grparams.random_scalar();
        let Us: Vec<G1Projective> = us.iter().map(|u| grparams.gen1() * u).collect();
        let Y = grparams.gen2() * y;
        let Ws: Vec<G2Projective> = ws.iter().map(|w| grparams.gen2() * w).collect();
        let zz = grparams.gen2() * z;

        let sps_vk = SPSVerificationKey {
            grparams: grparams.clone(),
            uus: Us,
            wws: Ws,
            yy: Y,
            zz,
        };
        let sps_sk = SPSSecretKey {
            spsVK: sps_vk.clone(),
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
    rr: G1Projective,
    ss: G1Projective,
    tt: G2Projective,
}
