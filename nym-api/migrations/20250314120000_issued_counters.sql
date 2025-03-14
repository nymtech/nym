/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: GPL-3.0-only
 */

CREATE TABLE issued_ticketbooks_count
(
--     keep those two values in the same table so we'd be able to see, for example, what different expiration dates were used on given issuance day
    issuance_date   DATE    NOT NULL DEFAULT CURRENT_DATE,
    expiration_date DATE    NOT NULL,
    count           INTEGER NOT NULL,

    UNIQUE (issuance_date, expiration_date)
);