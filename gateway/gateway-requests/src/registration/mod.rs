// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod handshake;

// TODO: is it perhaps possible to replace the 'custom' handshake with an existing
// implementation like with one of the variants on the Noise framework?

// Right now it's based on the STS (Station-to-Station) Protocol.
