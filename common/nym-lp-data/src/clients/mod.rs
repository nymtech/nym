// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod driver;
pub mod helpers;
pub mod traits;
pub mod types;

pub trait InputOptions<NdId>: Clone {
    fn reliability(&self) -> bool;
    fn routing_security(&self) -> bool;
    fn obfuscation(&self) -> bool;

    fn next_hop(&self) -> NdId;
}
