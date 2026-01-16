// Copyright 2025-2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_crypto::asymmetric::x25519;

// declare LP keys as type aliases to x25519 keys
pub type PrivateKey = x25519::PrivateKey;
pub type PublicKey = x25519::PublicKey;
pub type KeyPair = x25519::KeyPair;
