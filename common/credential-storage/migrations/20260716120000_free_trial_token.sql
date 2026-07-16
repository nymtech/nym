/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- Free-tier capability tokens. Deliberately separate from `emergency_credential`:
-- this is promotional access, not a network-emergency fallback.
CREATE TABLE free_trial_token
(
    id         INTEGER   NOT NULL PRIMARY KEY AUTOINCREMENT,
    token      TEXT      NOT NULL,
    expiration TIMESTAMP WITHOUT TIME ZONE NOT NULL
);
