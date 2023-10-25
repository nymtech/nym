use std::collections::HashMap;

use bls12_381::{G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Scalar};
use ff::Field;
use group::Curve;
use rand::thread_rng;

use crate::error::{CompactEcashError, Result};
use crate::utils::{hash_g1, try_deserialize_g2_projective, Signature};

const ATTRIBUTES_LEN: usize = 3;

pub struct GroupParameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Projective>,
    // Additional generator of the G1 group
    delta: G1Projective,
    /// Precomputed G2 generator used for the miller loop
    _g2_prepared_miller: G2Prepared,
}

impl GroupParameters {
    pub fn new() -> Result<GroupParameters> {
        let gammas = (1..=ATTRIBUTES_LEN)
            .map(|i| hash_g1(format!("gamma{}", i)))
            .collect();

        let delta = hash_g1("delta");

        Ok(GroupParameters {
            g1: G1Affine::generator(),
            g2: G2Affine::generator(),
            gammas,
            delta,
            _g2_prepared_miller: G2Prepared::from(G2Affine::generator()),
        })
    }

    pub(crate) fn gen1(&self) -> &G1Affine {
        &self.g1
    }

    pub(crate) fn gen2(&self) -> &G2Affine {
        &self.g2
    }

    pub(crate) fn gammas(&self) -> &Vec<G1Projective> {
        &self.gammas
    }

    pub(crate) fn gamma1(&self) -> &G1Projective {
        &self.gammas[0]
    }

    pub(crate) fn gamma2(&self) -> Option<&G1Projective> {
        self.gammas.get(2)
    }

    pub(crate) fn delta(&self) -> &G1Projective {
        &self.delta
    }

    pub fn random_scalar(&self) -> Scalar {
        // lazily-initialized thread-local random number generator, seeded by the system
        let mut rng = thread_rng();
        Scalar::random(&mut rng)
    }

    pub fn n_random_scalars(&self, n: usize) -> Vec<Scalar> {
        (0..n).map(|_| self.random_scalar()).collect()
    }

    pub(crate) fn prepared_miller_g2(&self) -> &G2Prepared {
        &self._g2_prepared_miller
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SecretKeyRP {
    pub(crate) x: Scalar,
    pub(crate) y: Scalar,
}

impl SecretKeyRP {
    pub fn public_key(&self, params: &GroupParameters) -> PublicKeyRP {
        PublicKeyRP {
            alpha: params.gen2() * self.x,
            beta: params.gen2() * self.y,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PublicKeyRP {
    pub(crate) alpha: G2Projective,
    pub(crate) beta: G2Projective,
}

pub struct Parameters {
    /// group parameters
    grp: GroupParameters,
    /// Public Key for range proof verification
    pk_rp: PublicKeyRP,
    /// Max value of wallet
    L: u64,
    /// list of signatures for values l in [0, L]
    signs: HashMap<u64, Signature>,
}

impl Parameters {
    pub fn grp(&self) -> &GroupParameters {
        &self.grp
    }
    pub fn pk_rp(&self) -> &PublicKeyRP {
        &self.pk_rp
    }
    pub fn L(&self) -> u64 {
        self.L
    }
    pub fn signs(&self) -> &HashMap<u64, Signature> {
        &self.signs
    }
    pub fn get_sign_by_idx(&self, idx: u64) -> Result<&Signature> {
        match self.signs.get(&idx) {
            Some(val) => return Ok(val),
            None => {
                return Err(CompactEcashError::RangeProofOutOfBound(
                    "Cannot find the range proof signature for the given value. \
                        Check if the requested value is within the bound 0..L"
                        .to_string(),
                ));
            }
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        //we omit grp as it is fixed
        let pk_rp_alpha_bytes = self.pk_rp.alpha.to_affine().to_compressed();
        let pk_rp_beta_bytes = self.pk_rp.beta.to_affine().to_compressed();
        let l_bytes = self.L.to_be_bytes();

        let signs_bytes: Vec<u8> = self
            .signs
            .iter()
            .flat_map(|(_, value)| value.to_bytes())
            .collect();

        let mut bytes = Vec::with_capacity(96 + 96 + 8 + signs_bytes.len());

        bytes.extend_from_slice(&pk_rp_alpha_bytes);
        bytes.extend_from_slice(&pk_rp_beta_bytes);
        bytes.extend_from_slice(&l_bytes);
        bytes.extend_from_slice(&signs_bytes);

        bytes
    }
}

impl TryFrom<&[u8]> for Parameters {
    type Error = CompactEcashError;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 96 + 96 + 8 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for Parameters deserialization".to_string(),
            ));
        }
        let pk_rp_alpha_bytes: [u8; 96] = bytes[..96].try_into().unwrap();
        let pk_rp_beta_bytes: [u8; 96] = bytes[96..192].try_into().unwrap();
        let ll = u64::from_be_bytes(bytes[192..200].try_into().unwrap());

        let mut index = 200;
        if bytes[index..].len() != ll as usize * 96 {
            return Err(CompactEcashError::Deserialization(
                "Invalid byte array for Parameters signatures deserialization".to_string(),
            ));
        }

        let pk_rp_alpha = try_deserialize_g2_projective(
            &pk_rp_alpha_bytes,
            CompactEcashError::Deserialization("Failed to deserialize pk_rp_alpha".to_string()),
        )?;

        let pk_rp_beta = try_deserialize_g2_projective(
            &pk_rp_beta_bytes,
            CompactEcashError::Deserialization("Failed to deserialize pk_rp_beta".to_string()),
        )?;

        let grp_params = GroupParameters::new()?;
        let pk_rp = PublicKeyRP {
            alpha: pk_rp_alpha,
            beta: pk_rp_beta,
        };

        let mut signs = HashMap::new();

        for l in 0..ll {
            let sign = Signature::try_from(&bytes[index..index + 96])?;
            signs.insert(l, sign);
            index += 96;
        }

        Ok(Self {
            grp: grp_params,
            pk_rp,
            L: ll,
            signs,
        })
    }
}
pub fn setup(L: u64) -> Parameters {
    let grp = GroupParameters::new().unwrap();
    let x = grp.random_scalar();
    let y = grp.random_scalar();
    let sk_rp = SecretKeyRP { x, y };
    let pk_rp = sk_rp.public_key(&grp);
    let mut signs = HashMap::new();
    for l in 0..L {
        let r = grp.random_scalar();
        let h = grp.gen1() * r;
        signs.insert(
            l,
            Signature {
                0: h,
                1: h * (x + y * Scalar::from(l)),
            },
        );
    }
    Parameters {
        grp,
        pk_rp,
        L,
        signs,
    }
}
