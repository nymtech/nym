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

#[cfg(any(feature = "asymmetric", feature = "minimum-asymmetric"))]
pub mod asymmetric;
#[cfg(feature = "digest-feature")]
pub mod crypto_hash;
#[cfg(feature = "hkdf-feature")]
pub mod hkdf;
#[cfg(feature = "hmac-feature")]
pub mod hmac;
#[cfg(feature = "shared-keys")]
pub mod shared_key;
#[cfg(feature = "symmetric")]
pub mod symmetric;

#[cfg(any(
    feature = "digest-feature",
    feature = "hkdf-feature",
    feature = "hmac-feature"
))]
pub use digest::Digest;
#[cfg(any(
    feature = "digest-feature",
    feature = "hkdf-feature",
    feature = "hmac-feature",
    feature = "symmetric",
    feature = "shared_keys"
))]
pub use generic_array;

// with the below my idea was to try to introduce having a single place of importing all hashing, encryption,
// etc. algorithms and import them elsewhere as needed via common/crypto
#[cfg(feature = "symmetric")]
pub use aes_ctr;
#[cfg(any(
    feature = "digest-feature",
    feature = "hkdf-feature",
    feature = "hmac-feature"
))]
pub use blake3;

// TODO: this function uses all three modules: asymmetric crypto, symmetric crypto and derives key...,
// so I don't know where to put it...
