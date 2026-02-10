/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE wireguard_peer
    ADD COLUMN psk VARCHAR;