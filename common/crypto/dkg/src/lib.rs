// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// forward-secure public key encryption scheme
pub mod bte;
pub mod error;
pub mod interpolation;

// this entire module is a big placeholder for whatever scheme we decide to use for the
// secure channel encryption scheme, but I would assume that the top-level API would
// remain more or less the same
pub mod dealing;
pub(crate) mod share;
pub(crate) mod utils;

pub use dealing::*;
pub use share::*;

// TODO: presumably this should live in a some different, common, crate?
pub type Threshold = u64;
pub type NodeIndex = u64;

#[cfg(test)]
mod tests {
    use crate::interpolation::perform_lagrangian_interpolation_at_origin;
    use crate::interpolation::polynomial::Polynomial;
    use bls12_381::Scalar;
    use rand_chacha::rand_core::SeedableRng;

    #[test]
    fn basic_dummy_secret_sharing() {
        let degree = 2;

        let dummy_seed = [1u8; 32];
        let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);

        let p1 = Polynomial::new_random(&mut rng, degree);
        let p2 = Polynomial::new_random(&mut rng, degree);
        let p3 = Polynomial::new_random(&mut rng, degree);
        let p4 = Polynomial::new_random(&mut rng, degree);

        let zero = Scalar::zero();
        let one = Scalar::one();
        let two = Scalar::from(2);
        let three = Scalar::from(3);
        let four = Scalar::from(4);

        // i.e. given:
        // p1 = a1 + x * b1 + ...
        // p2 = a2 + x * b2 + ...
        // ...
        // expected = (a1 + a2 + ...) + x * (b1 + b2 + ...) + ...
        // note: master polynomial is NEVER explicitly computed
        let expected_master = &p1 + &p2 + &p3 + &p4;

        let v1_secret = p1.evaluate_at(&one)
            + p2.evaluate_at(&one)
            + p3.evaluate_at(&one)
            + p4.evaluate_at(&one);
        let v2_secret = p1.evaluate_at(&two)
            + p2.evaluate_at(&two)
            + p3.evaluate_at(&two)
            + p4.evaluate_at(&two);
        let v3_secret = p1.evaluate_at(&three)
            + p2.evaluate_at(&three)
            + p3.evaluate_at(&three)
            + p4.evaluate_at(&three);
        let v4_secret = p1.evaluate_at(&four)
            + p2.evaluate_at(&four)
            + p3.evaluate_at(&four)
            + p4.evaluate_at(&four);

        // note that the following would have never happened in actual dkg setting, but it's
        // used here mostly for a sanity check on the maths used
        let samples = vec![
            (one, v1_secret),
            (two, v2_secret),
            (three, v3_secret),
            (four, v4_secret),
        ];
        let master_secret = perform_lagrangian_interpolation_at_origin(&samples).unwrap();

        assert_eq!(expected_master.evaluate_at(&zero), master_secret);
        assert_eq!(expected_master.evaluate_at(&one), v1_secret);
        assert_eq!(expected_master.evaluate_at(&two), v2_secret);
        assert_eq!(expected_master.evaluate_at(&three), v3_secret);
        assert_eq!(expected_master.evaluate_at(&four), v4_secret);

        // since we have 4 parties, but polynomials used are of degree 2, we only need at least 3
        // issuers to contribute
        let samples2 = vec![(one, v1_secret), (three, v3_secret), (four, v4_secret)];
        let master_secret2 = perform_lagrangian_interpolation_at_origin(&samples2).unwrap();
        assert_eq!(master_secret, master_secret2)
    }
}
