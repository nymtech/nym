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
use crypto::hmac::{hmac::Mac, HmacOutput};
use nymsphinx::params::GatewayIntegrityHmacAlgorithm;

pub mod authentication;
pub mod registration;
pub mod types;

pub const DUMMY_MESSAGE_CONTENT: &[u8] =
    b"[DUMMY MESSAGE] Wanting something does not give you the right to have it.";

pub use crypto::generic_array;
pub use types::*;

pub type GatewayMac = HmacOutput<GatewayIntegrityHmacAlgorithm>;

// TODO: could using `Mac` trait here for OutputSize backfire?
// Should hmac itself be exposed, imported and used instead?
pub type GatewayMacSize = <GatewayIntegrityHmacAlgorithm as Mac>::OutputSize;
