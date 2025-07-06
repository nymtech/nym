/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE watcher_execution
(
    start         TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    end           TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    error_message TEXT
)