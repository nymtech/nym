// Copyright 2020 Nym Technologies SA
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

use digest::{BlockInput, FixedOutput, Reset, Update};
use generic_array::ArrayLength;
use hkdf::Hkdf;

#[derive(Debug)]
pub enum HkdfError {
    InvalidOkmLength,
}

/// Perform HKDF `extract` then `expand` as a single step.
pub fn extract_then_expand<D>(
    salt: Option<&[u8]>,
    ikm: &[u8],
    info: Option<&[u8]>,
    okm_length: usize,
) -> Result<Vec<u8>, HkdfError>
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    // TODO: this would need to change if we ever needed the generated pseudorandom key, but
    // realistically I don't see any reasons why we might need it

    let hkdf = Hkdf::<D>::new(salt, ikm);
    let mut okm = vec![0u8; okm_length];
    if hkdf.expand(info.unwrap_or_else(|| &[]), &mut okm).is_err() {
        return Err(HkdfError::InvalidOkmLength);
    }

    Ok(okm)
}
