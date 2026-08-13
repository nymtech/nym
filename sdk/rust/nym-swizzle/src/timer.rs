// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The single cfg-gated point where the crate touches wall-clock time, so the
//! rest of the code is target-agnostic.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use tokio::time::sleep;

#[cfg(target_arch = "wasm32")]
pub(crate) use wasmtimer::tokio::sleep;
