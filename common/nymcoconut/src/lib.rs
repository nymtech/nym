// Copyright 2021 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::convert::TryInto;

use bls12_381::Scalar;

pub use elgamal::elgamal_keygen;
pub use elgamal::ElGamalKeyPair;
pub use elgamal::PublicKey;
pub use error::CoconutError;
pub use scheme::aggregation::aggregate_signature_shares;
pub use scheme::aggregation::aggregate_verification_keys;
pub use scheme::issuance::blind_sign;
pub use scheme::issuance::prepare_blind_sign;
pub use scheme::issuance::BlindSignRequest;
pub use scheme::keygen::ttp_keygen;
pub use scheme::keygen::KeyPair;
pub use scheme::keygen::VerificationKey;
pub use scheme::setup::setup;
pub use scheme::setup::Parameters;
pub use scheme::verification::prove_credential;
pub use scheme::verification::verify_credential;
pub use scheme::verification::Theta;
pub use scheme::BlindedSignature;
pub use scheme::Signature;
pub use scheme::SignatureShare;
pub use traits::Base58;
pub use utils::hash_to_scalar;

use crate::traits::Bytable;

mod constants;
pub mod elgamal;
mod error;
mod impls;
mod proofs;
mod scheme;
#[cfg(test)]
mod tests;
mod traits;
mod utils;

pub type Attribute = Scalar;
pub type PrivateAttribute = Attribute;
pub type PublicAttribute = Attribute;

impl Bytable for Attribute {
    fn to_byte_vec(&self) -> Vec<u8> {
        self.to_bytes().to_vec()
    }

    fn try_from_byte_slice(slice: &[u8]) -> Result<Self, CoconutError> {
        Ok(Attribute::from_bytes(slice.try_into().unwrap()).unwrap())
    }
}

impl Base58 for Attribute {}
