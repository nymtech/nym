// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// TODO: if this becomes too cumbersome, perhaps consider a more streamlined solution like tarpc
// (I wouldn't have needed that for DKG, but if this is to be used for different purposes, maybe
// it would have been more appropriate)

pub(crate) mod codec;
pub(crate) mod error;
pub(crate) mod message;
pub(crate) mod receiver;
pub(crate) mod sender;

pub(crate) const PROTOCOL_VERSION: u32 = 1;
