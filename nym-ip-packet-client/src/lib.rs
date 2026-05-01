// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

//! Anonymous IPR connect uses **v8** on the wire so exits that reject non-stream v9 still answer.
//! **v9** is re-exported for code paths that use LP Stream framing. Incoming IPR responses may be **v8 or v9** (same bincode shape).

mod connect;
mod error;
mod helpers;
mod listener;

pub use connect::IprClientConnect;
pub use error::Error;
pub use listener::{IprListener, MixnetMessageOutcome};

pub use nym_ip_packet_requests::v8;
pub use nym_ip_packet_requests::v9;

pub use v8 as current;
