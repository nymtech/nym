use bls12_381::{G1Affine, G2Affine};

pub struct Parameters {
    /// Generator of the G1 group
    g1: G1Affine,
    /// Generator of the G2 group
    g2: G2Affine,
    /// Additional generators of the G1 group
    gammas: Vec<G1Affine>,
    /// Value of wallet
    L: usize,
}