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

pub mod encryption;
pub mod identity;

// TODO: ideally those trait should be moved to 'pemstore' crate, however, that would cause
// circular dependency. The best solution would be to remove dependency on 'crypto' from
// pemstore by using either dynamic dispatch or generics - perhaps this should be done
// at some point during one of refactors.

pub trait PemStorableKey {
    fn pem_type(&self) -> String;
    fn to_bytes(&self) -> Vec<u8>;
}

pub trait PemStorableKeyPair: Sized {
    type PrivatePemKey: PemStorableKey;
    type PublicPemKey: PemStorableKey;
    type Error: std::error::Error;

    fn private_key(&self) -> &Self::PrivatePemKey;
    fn public_key(&self) -> &Self::PublicPemKey;

    fn from_bytes(priv_bytes: &[u8], pub_bytes: &[u8]) -> Result<Self, Self::Error>;
}
