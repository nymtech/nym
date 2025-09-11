/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE blinded_shares
(
    id            INTEGER                     NOT NULL PRIMARY KEY,
    status        TEXT                        NOT NULL,
    device_id     TEXT                        NOT NULL,
    credential_id TEXT                        NOT NULL,
    data          TEXT DEFAULT NULL,
    error_message TEXT DEFAULT NULL,
    created       TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated       TIMESTAMP WITHOUT TIME ZONE NOT NULL
);

CREATE UNIQUE INDEX blinded_shares_index ON blinded_shares (credential_id, device_id);

