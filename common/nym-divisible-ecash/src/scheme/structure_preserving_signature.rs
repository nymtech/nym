use std::convert::TryFrom;

use bls12_381::{G1Projective, G2Projective, Scalar};
use group::Curve;

use crate::Attribute;
use crate::scheme::setup::GroupParameters;

#[derive(Debug, Clone)]
pub(crate) struct SPSVerificationKey {
    pub grparams: GroupParameters,
    pub Us: Vec<G1Projective>,
    pub Ws: Vec<G2Projective>,
    pub Y: G2Projective,
    pub Z: G2Projective,
}

pub(crate) struct SPSSecretKey {
    spsVK: SPSVerificationKey,
    us: Vec<Scalar>,
    ws: Vec<Scalar>,
    y: Scalar,
    z: Scalar,
}

impl SPSSecretKey {
    pub fn z(&self) -> Scalar { self.z }
    pub fn y(&self) -> Scalar { self.y }
    pub fn sign(&self, grparams: GroupParameters, attributes: Vec<Attribute>) -> SPSSignature {
        let r = grparams.random_scalar();
        let R = grparams.gen1() * r;
        let prod: Vec<Scalar> = attributes.iter().zip(self.ws.iter()).map(|(w_i, m_i)| m_i * w_i.neg()).collect();
        let Z = grparams.gen1() * (self.z() - r * self.y()) + prod.iter().fold(1 | acc, x | acc * x);
        // let sum = a.iter().fold(0, |acc, x| acc + x);
        // let Z: G1Projective = grparams.gen1() * (self.z() - r * self.y())
        //     + attributes
        //     .iter()
        //     .zip(self.ws.iter())
        //     .map(|(w_i, m_i)| m_i * w_i.neg()).product();
        SPSSignature {}
    }
}

pub struct SPSKeyPair {
    spsSK: SPSSecretKey,
    spsVK: SPSVerificationKey,
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
        let Z = grparams.gen2() * z;

        let spsVK = SPSVerificationKey {
            grparams,
            Us,
            Ws,
            Y,
            Z,
        };
        let spsSK = SPSSecretKey {
            spsVK: spsVK.clone(),
            us,
            ws,
            y,
            z,
        };
        SPSKeyPair { spsSK, spsVK }
    }
}

pub struct SPSSignature {}

