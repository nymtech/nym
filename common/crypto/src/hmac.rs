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
use hmac::{crypto_mac, Hmac, Mac, NewMac};

/// Compute keyed hmac
pub fn compute_keyed_hmac<D>(key: &[u8], data: &[u8]) -> crypto_mac::Output<Hmac<D>>
where
    D: Update + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8>,
    D::OutputSize: ArrayLength<u8>,
{
    let mut hmac =
        Hmac::<D>::new_varkey(key).expect("HMAC should be able to take key of any size!");
    hmac.update(data);
    hmac.finalize()
}
