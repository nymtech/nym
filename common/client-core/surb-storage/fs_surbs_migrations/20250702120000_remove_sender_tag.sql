/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- don't persist sender_tag in the DB. instead generate fresh one on each restart
-- this will:
-- A) further help against correlation attacks
-- B) realistically after client restarts, we might be in new key rotation anyway meaning receiver would have to start
-- "from scratch" with surbs

DROP TABLE sender_tag;