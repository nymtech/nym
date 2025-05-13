/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

ALTER TABLE sessions_active
    ADD COLUMN remember INTEGER NOT NULL default 0;