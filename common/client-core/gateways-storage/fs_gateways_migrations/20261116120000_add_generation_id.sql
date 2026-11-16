/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- NULL marks a registration that uses the client's root keys
ALTER TABLE remote_gateway_details ADD COLUMN generation_id BLOB CHECK (generation_id IS NULL OR length(generation_id) = 16);
