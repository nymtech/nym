use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use rand::thread_rng;

use crate::constants::L;
use crate::error::Result;
use crate::scheme::structure_preserving_signature::{SPSKeyPair, SPSSignature, SPSVerificationKey};
use crate::utils::{hash_g1, hash_g2};

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
    grp: GroupParameters,
    params_u: ParametersUser,
    params_a: ParametersAuthority,
}

impl Parameters {
    pub(crate) fn get_grp(&self) -> &GroupParameters { &self.grp }

    pub(crate) fn get_params_u(&self) -> &ParametersUser { &self.params_u }

    pub(crate) fn get_params_a(&self) -> &ParametersAuthority { &self.params_a }

    pub fn new(grp: GroupParameters) -> Parameters {
        let g1 = grp.gen1();
        let g2 = grp.gen2();
        let psi1 = hash_g1("psi1");
        let psi2 = hash_g2("psi2");
        let gamma1 = hash_g1("gamma1");
        let gamma2 = hash_g1("gamma2");
        let eta = hash_g1("eta");
        let omega = hash_g1("omega");

        let z = grp.random_scalar();
        let y = grp.random_scalar();

        let vec_a = grp.n_random_scalars(L as usize);

        let sigma = g1 * z;
        let theta = eta * z;

        let sigmas_u: Vec<G1Projective> = (1..=L)
            .map(|i| sigma * (y * Scalar::from(i)))
            .collect();

        let thetas_u: Vec<G1Projective> = (1..=L)
            .map(|i| theta * (y * Scalar::from(i)))
            .collect();

        let deltas_a: Vec<G2Projective> = (0..=L - 1)
            .map(|i| g2 * (y * Scalar::from(i)))
            .collect();
        let etas_u: Vec<G1Projective> = vec_a.iter().map(|x| g1 * x).collect();

        let mut etas_a: Vec<G2Projective> = Default::default();
        for l in 1..=L {
            println!("l = {:?}", l);
            for k in 0..=l - 1 {
                println!("k = {:?}", k);
                etas_a.push(g2 * (vec_a[l as usize - 1].neg() * (y * Scalar::from(k))));
            }
        }

        let sps_keypair = SPSKeyPair::new(grp.clone(), 2, 0);
        let messages_a = vec![sigma, theta];

        let sps_signatures: Vec<SPSSignature> = sigmas_u
            .iter()
            .zip(thetas_u.iter())
            .map(|(sigma, theta)| sps_keypair.sps_sk.sign(grp.clone(), Some(&messages_a), None))
            .collect();

        // Compute signature for each pair sigma, theta
        let params_u = ParametersUser {
            gammas: vec![gamma1, gamma2],
            psi1,
            psi2,
            eta,
            omega,
            etas: etas_u,
            sigmas: sigmas_u,
            thetas: thetas_u,
            sps_signatures,
            sps_pk: sps_keypair.sps_vk,
        };

        let params_a = ParametersAuthority {
            deltas: deltas_a,
            etas: etas_a,
        };

        return Parameters {
            grp,
            params_u,
            params_a,
        };
    }
}

#[derive(Debug, Clone)]
pub struct ParametersUser {
    gammas: Vec<G1Projective>,
    psi1: G1Projective,
    psi2: G2Projective,
    eta: G1Projective,
    omega: G1Projective,
    etas: Vec<G1Projective>,
    sigmas: Vec<G1Projective>,
    thetas: Vec<G1Projective>,
    sps_signatures: Vec<SPSSignature>,
    sps_pk: SPSVerificationKey,
}

impl ParametersUser {
    pub(crate) fn get_gammas(&self) -> &Vec<G1Projective> { &self.gammas }

    pub(crate) fn get_psi0(&self) -> &G1Projective { &self.psi1 }

    pub(crate) fn get_psi1(&self) -> &G2Projective { &self.psi2 }

    pub(crate) fn get_eta(&self) -> &G1Projective { &self.eta }

    pub(crate) fn get_omega(&self) -> &G1Projective { &self.omega }

    pub(crate) fn get_etas(&self) -> &[G1Projective] { &self.etas }

    pub(crate) fn get_ith_eta(&self, idx: usize) -> &G1Projective { self.etas.get(idx).unwrap() }

    pub(crate) fn get_sigmas(&self) -> &[G1Projective] { &self.sigmas }

    pub(crate) fn get_ith_sigma(&self, idx: usize) -> &G1Projective { self.sigmas.get(idx).unwrap() }

    pub(crate) fn get_thetas(&self) -> &[G1Projective] { &self.thetas }

    pub(crate) fn get_ith_theta(&self, idx: usize) -> &G1Projective { self.thetas.get(idx).unwrap() }

    pub(crate) fn get_sps_signs(&self) -> &[SPSSignature] { &self.sps_signatures }

    pub(crate) fn get_ith_sps_sign(&self, idx: usize) -> &SPSSignature { &self.sps_signatures.get(idx).unwrap() }

    pub(crate) fn get_sps_pk(&self) -> &SPSVerificationKey { &self.sps_pk }
}

#[derive(Debug, Clone)]
pub struct ParametersAuthority {
    deltas: Vec<G2Projective>,
    etas: Vec<G2Projective>,
}

impl ParametersAuthority {
    pub(crate) fn get_deltas(&self) -> &[G2Projective] { &self.deltas }

    pub(crate) fn get_ith_delta(&self, idx: usize) -> &G2Projective { self.deltas.get(idx).unwrap() }

    pub(crate) fn get_etas(&self) -> &[G2Projective] { &self.etas }

    pub(crate) fn get_ith_eta(&self, idx: usize) -> &G2Projective { self.etas.get(idx).unwrap() }
}
