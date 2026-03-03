// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

/// Maximum UDP packet size we'll accept
/// Sphinx packets are typically ~2KB, LP overhead is ~50 bytes, so 4KB is plenty
const MAX_UDP_PACKET_SIZE: usize = 4096;

pub mod handler;
pub(crate) mod listener;

#[cfg(test)]
mod tests {
    use super::*;

    // Sphinx packets are typically around 2KB
    // 4KB should be plenty with room to spare
    const _: () = {
        assert!(MAX_UDP_PACKET_SIZE >= 2048 + 100);
    };
}
