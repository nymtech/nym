// Copyright 2024 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use rand::rngs::OsRng;
use rand::RngCore;
use uuid::Uuid;

pub fn random_uuid() -> Uuid {
    let mut bytes = [0u8; 16];
    let mut rng = OsRng;
    rng.fill_bytes(&mut bytes);
    Uuid::from_bytes(bytes)
}
