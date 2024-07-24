/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

CREATE TABLE wireguard_peer
(
    public_key TEXT    NOT NULL PRIMARY KEY UNIQUE,
    preshared_key TEXT,
    protocol_version INTEGER,
    endpoint TEXT,
    last_handshake TIMESTAMP,
    tx_bytes BIGINT NOT NULL,
    rx_bytes BIGINT NOT NULL,
    persistent_keepalive_interval INTEGER,
    allowed_ips BLOB NOT NULL
);