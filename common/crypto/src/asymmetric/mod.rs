// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod ed25519;
pub mod x25519;

// don't break existing imports
// but deprecate them
#[deprecated(note = "use ed25519 instead")]
pub use ed25519 as identity;

#[deprecated(note = "use x25519 instead")]
pub use x25519 as encryption;
