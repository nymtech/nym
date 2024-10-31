/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */


DROP TABLE blinded_shares;
CREATE TABLE blinded_shares
(
    id               INTEGER                     NOT NULL PRIMARY KEY,
--    removed reference to `ticketbook_deposit` as the deposit wouldn't actually have been made before the pending share is inserted
    request_uuid     TEXT                        NOT NULL,
    status           TEXT                        NOT NULL,
    device_id        TEXT                        NOT NULL,
    credential_id    TEXT                        NOT NULL,
    available_shares INTEGER                     NOT NULL DEFAULT 0,
    error_message    TEXT                                 DEFAULT NULL,
    created          TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    updated          TIMESTAMP WITHOUT TIME ZONE NOT NULL
);