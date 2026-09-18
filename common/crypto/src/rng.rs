// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use rand010::rand_core::UnwrapErr;
pub use rand010::rngs::SysRng;

/// OS entropy as an infallible `CryptoRng`, panicking on entropy failure just as rand 0.8's
/// `OsRng` did. Unlike `rand010::rng()` it is `Send + Sync + Copy`, so this is the one to reach
/// for when the rng lives in a struct or across an `.await`; prefer `rand010::rng()` otherwise.
pub type OsRng = UnwrapErr<SysRng>;

/// Constructs an [`OsRng`].
pub fn os_rng() -> OsRng {
    UnwrapErr(SysRng)
}
