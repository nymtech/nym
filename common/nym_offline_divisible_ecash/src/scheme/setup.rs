use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use rand::thread_rng;

use crate::constants::L;
use crate::error::Result;
use crate::scheme::structure_preserving_signature::{SPSKeyPair, SPSSecretKey, SPSSignature, SPSVerificationKey};
use crate::utils::hash_g1;

#[derive(Debug, Clone)]
pub struct GroupParameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Precomputed G2 generator used for the miller loop
    _g2_prepared_miller: G2Prepared,
}

impl GroupParameters {
    pub fn new() -> Result<GroupParameters> {
        Ok(GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            _g2_prepared_miller: G2Prepared::from(G2Affine::generator()),
        })
    }

    pub(crate) fn gen1(&self) -> &G1Affine {
        &self.g1
    }

    pub(crate) fn gen2(&self) -> &G2Affine {
        &self.g2
    }

    pub(crate) fn prepared_miller_g2(&self) -> &G2Prepared {
        &self._g2_prepared_miller
    }

    pub(crate) fn random_scalar(&self) -> Scalar {
        // lazily-initialized thread-local random number generator, seeded by the system
        let mut rng = thread_rng();
        Scalar::random(&mut rng)
    }

    pub(crate) fn n_random_scalars(&self, n: usize) -> Vec<Scalar> {
        (0..n).map(|_| self.random_scalar()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct Parameters {
    paramsUser: ParametersUser,
    paramsAuth: ParametersAuthority,
}

impl Parameters {
    pub fn new(grp: GroupParameters) -> Parameters {
        let g1 = grp.gen1();
        let g2 = grp.gen2();
        let gamma1 = hash_g1("gamma1");
        let gamma2 = hash_g1("gamma2");
        let eta = hash_g1("eta");
        let omega = hash_g1("omega");

        let z = grp.random_scalar();
        let y = grp.random_scalar();

        let vec_a = grp.n_random_scalars(L as usize);

        let sigma = g1 * z;
        let theta = eta * z;

        let sigmasUser: Vec<G1Projective> = (1..=L)
            .map(|i| sigma * (y * Scalar::from(i)))
            .collect();
        let thetasUser: Vec<G1Projective> = (1..=L)
            .map(|i| theta * (y * Scalar::from(i)))
            .collect();

        let deltasAuth: Vec<G2Projective> = (0..=L - 1)
            .map(|i| g2 * (y * Scalar::from(i)))
            .collect();
        let etasUser: Vec<G1Projective> = vec_a.iter().map(|x| g1 * x).collect();

        let mut etasAuth: Vec<G2Projective> = Default::default();
        for l in 1..=L {
            for k in 0..=l - 1 {
                etasAuth.push(g2 * (vec_a[l as usize].neg() * (y * Scalar::from(k))));
            }
        }

        let sps_keypair = SPSKeyPair::new(grp.clone(), 2, 0);
        let messagesA = vec![sigma, theta];

        let sps_signatures: Vec<SPSSignature> = sigmasUser
            .iter()
            .zip(thetasUser.iter())
            .map(|(sigma, theta)| sps_keypair.sps_sk.sign(grp.clone(), Some(&messagesA), None))
            .collect();

        // Compute signature for each pair sigma, theta
        let paramsUser = ParametersUser {
            g1: *g1,
            g2: *g2,
            gamma1,
            gamma2,
            eta,
            omega,
            etas: etasUser,
            sigmas: sigmasUser,
            thetas: thetasUser,
            sps_signatures,
            sps_pk: sps_keypair.sps_vk,
        };

        let paramsAuth = ParametersAuthority {
            deltas: deltasAuth,
            etas: etasAuth,
        };

        return Parameters {
            paramsUser,
            paramsAuth,
        };
    }
}

#[derive(Debug, Clone)]
pub struct ParametersUser {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    gamma1: G1Projective,
    gamma2: G1Projective,
    eta: G1Projective,
    omega: G1Projective,
    etas: Vec<G1Projective>,
    sigmas: Vec<G1Projective>,
    thetas: Vec<G1Projective>,
    sps_signatures: Vec<SPSSignature>,
    sps_pk: SPSVerificationKey,
}

#[derive(Debug, Clone)]
pub struct ParametersAuthority {
    deltas: Vec<G2Projective>,
    etas: Vec<G2Projective>,
}
