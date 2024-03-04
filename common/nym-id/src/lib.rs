// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

pub mod error;
pub mod import_credential;

pub use error::NymIdError;
pub use import_credential::import_credential;
