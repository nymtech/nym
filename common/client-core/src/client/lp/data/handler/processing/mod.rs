// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What a client does with an arriving frame, once the transport and framing layers have said what
//! kind it is.
//!
//! One module per [`ClientMessage`] variant, so a new frame kind is a new module and a new arm in [`process_unwrapped`]
//!
//! [`ClientMessage`]: super::messages::ClientMessage
//! [`process_unwrapped`]: nym_lp_data::clients::traits::ClientUnwrappingPipeline::process_unwrapped

pub(crate) mod sphinx;
