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

use hmac::{crypto_mac, Hmac, Mac, NewMac};

pub type Blake3hmac = crypto_mac::Output<Hmac<blake3::Hasher>>;

/// Compute keyed hmac using blake3 algorithm
pub fn compute_keyed_hmac(key: &[u8], data: &[u8]) -> Blake3hmac {
    let mut hmac = Hmac::<blake3::Hasher>::new_varkey(key)
        .expect("HMAC should be able to take key of any size!");
    hmac.update(data);
    hmac.finalize()
}
