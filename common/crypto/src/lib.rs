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

pub mod asymmetric;
pub mod crypto_hash;
pub mod hkdf;
pub mod hmac;
pub mod shared_key;
pub mod symmetric;

pub use digest::Digest;
pub use generic_array;

// with the below my idea was to try to introduce having a single place of importing all hashing, encryption,
// etc. algorithms and import them elsewhere as needed via common/crypto
pub use aes_ctr;
pub use blake3;

// TODO: this function uses all three modules: asymmetric crypto, symmetric crypto and derives key...,
// so I don't know where to put it...
