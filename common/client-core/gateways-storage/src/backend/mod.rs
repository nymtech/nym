// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-gateways-storage"))]
pub mod fs_backend;

pub mod mem_backend;
