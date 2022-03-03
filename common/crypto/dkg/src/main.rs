// use crate::bte::{BabyStepGiantStepLookup, CHUNK_MAX};
// use bls12_381::{Gt, Scalar};
// use error::DkgError;
// use group::Group;
// use rand_core::{RngCore, SeedableRng};
//
// pub(crate) mod bte;
// pub(crate) mod error;
//
// // 500 ms for 2 byte
// // 20s for 4 byte
//
// fn main() {
//     let dummy_seed = [1u8; 32];
//     let mut rng = rand_chacha::ChaCha20Rng::from_seed(dummy_seed);
//
//     let base = Gt::random(&mut rng);
//     let table = BabyStepGiantStepLookup::precompute(&base);
//
//     let now = std::time::SystemTime::now();
//
//     for i in 0u64..100 {
//         println!("{}", i);
//         let x = (rng.next_u32() as u64) % CHUNK_MAX as u64;
//         let target = base * Scalar::from(x);
//
//         assert_eq!(
//             bte::baby_step_giant_step(&target, &base, Some(&table)).unwrap(),
//             x as bte::Chunk
//         );
//     }
//
//     let elapsed = std::time::SystemTime::now();
//     println!("took {:?}", elapsed.duration_since(now));
// }
