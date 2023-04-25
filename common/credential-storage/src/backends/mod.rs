// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod memory;
#[cfg(not(target_arch = "wasm32"))]
pub mod sqlite;
