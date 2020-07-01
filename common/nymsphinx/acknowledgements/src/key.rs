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

use crypto::symmetric::aes_ctr::Aes128Key;
use rand::{CryptoRng, RngCore};
use std::ops::Deref;

pub struct AckAes128Key(Aes128Key);

impl AckAes128Key {
    pub fn new<R: RngCore + CryptoRng>(rng: &mut R) -> Self {
        AckAes128Key(crypto::symmetric::aes_ctr::generate_key(rng))
    }

    // TODO: is there any possible use of storing this on disk and this adding to/from bytes methods?
}

impl Deref for AckAes128Key {
    type Target = Aes128Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
