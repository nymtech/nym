// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use service_providers_common::define_simple_version;

/// Defines initial version of the communication interface between socks5 clients and
/// network requesters (socks5).
// note: we start from '3' so that we could distinguish cases where no version is provided
// and legacy communication mode is used instead
pub const INITIAL_INTERFACE_VERSION: u8 = 3;

/// Defines the current version of the communication interface between socks5 clients and
/// network requesters (socks5).
/// It has to be incremented for any breaking change.
pub const INTERFACE_VERSION: u8 = 3;

define_simple_version!(
    Socks5ProtocolVersion,
    INITIAL_INTERFACE_VERSION,
    INTERFACE_VERSION
);
