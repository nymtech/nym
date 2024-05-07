/*
 * Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */


ALTER TABLE available_bandwidth
RENAME COLUMN freepass_expiration TO expiration;