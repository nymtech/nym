// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "coconut")]
pub mod coconut;
pub mod error;
#[cfg(not(feature = "coconut"))]
pub mod token;

#[cfg(feature = "coconut")]
pub use coconut::utils::{obtain_aggregate_signature, obtain_aggregate_verification_key};
