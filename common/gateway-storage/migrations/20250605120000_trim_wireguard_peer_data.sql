/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

DELETE FROM wireguard_peer WHERE client_id IS NULL;

CREATE TABLE wireguard_peer_new
(
    public_key                      TEXT                            NOT NULL PRIMARY KEY UNIQUE,
    allowed_ips                     BLOB                            NOT NULL,
    client_id                       INTEGER REFERENCES clients(id)  NOT NULL
);

INSERT INTO wireguard_peer_new (public_key, allowed_ips, client_id)
SELECT public_key, allowed_ips, client_id FROM wireguard_peer;

DROP TABLE wireguard_peer;
ALTER TABLE wireguard_peer_new RENAME TO wireguard_peer;