use bls12_381::{G1Projective, G2Prepared, G2Projective, pairing, Scalar};

pub use scheme::aggregation::aggregate_verification_keys;
pub use scheme::aggregation::aggregate_wallets;
pub use scheme::identification;
pub use scheme::keygen::{PublicKeyUser, SecretKeyUser, VerificationKeyAuth};
pub use scheme::keygen::ttp_keygen_authorities;
pub use scheme::keygen::ttp_keygen_users;
pub use scheme::PartialWallet;
pub use scheme::PayInfo;
pub use scheme::setup;
pub use scheme::withdrawal::issue;
pub use scheme::withdrawal::issue_verify;
pub use scheme::withdrawal::withdrawal_request;
pub use traits::Base58;

use crate::error::DivisibleEcashError;
use crate::traits::Bytable;

mod error;
mod proofs;
mod scheme;
#[cfg(test)]
mod tests;
mod traits;
mod utils;
mod constants;

pub type Attribute = Scalar;
