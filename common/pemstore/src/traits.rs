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

pub trait PemStorableKey: Sized {
    type Error: std::error::Error;
    fn pem_type() -> &'static str;
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self, Self::Error>;
}

pub trait PemStorableKeyPair {
    type PrivatePemKey: PemStorableKey;
    type PublicPemKey: PemStorableKey;

    fn private_key(&self) -> &Self::PrivatePemKey;
    fn public_key(&self) -> &Self::PublicPemKey;
    fn from_keys(private_key: Self::PrivatePemKey, public_key: Self::PublicPemKey) -> Self;
}
