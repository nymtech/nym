use crate::{G1Point, G2Point};

pub trait SomeRngTrait {}

// pub struct Params<R: SomeRngTrait> {
pub struct Params<R: SomeRngTrait> {
    group_order: (), // presumably Scalar?
    gen1: G1Point,
    gen2: G2Point,
    hs: Vec<G1Point>,
    rng: R, // putting rng here, believe me, will make stuff way cleaner because you need to generate
            // pseudo-random stuff almost in every single step
            // I kinda seem to recall initially not doing that in Go and regretting it later on
}
