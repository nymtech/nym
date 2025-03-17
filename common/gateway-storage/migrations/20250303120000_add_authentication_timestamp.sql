/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

ALTER TABLE shared_keys
    ADD COLUMN last_used_authentication TIMESTAMP WITHOUT TIME ZONE;