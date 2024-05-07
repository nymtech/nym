// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod memory;
#[cfg(all(not(target_arch = "wasm32"), feature = "persistent-storage"))]
pub mod sqlite;
