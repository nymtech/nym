/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- default value of 0 implies 'unknown' variant
ALTER TABLE reply_surb
    ADD COLUMN encoded_key_rotation TINYINT NOT NULL DEFAULT 0;