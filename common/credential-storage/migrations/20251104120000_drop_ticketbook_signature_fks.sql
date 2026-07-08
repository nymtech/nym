/*
 * Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- Best-effort ticketbook storage: a ticketbook may now be stored before its coin-index /
-- expiration-date signatures are available (those are fetched lazily at spend time). Previously
-- the `ecash_ticketbook` table foreign-keyed to those signature tables, so an insert would fail
-- if the signatures weren't present yet. SQLite can't drop a constraint in place, so the table is
-- recreated without those foreign keys (all columns/data/uniqueness are otherwise preserved).

CREATE TABLE ecash_ticketbook_new
(
    id                     INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    serialization_revision INTEGER NOT NULL,
    ticketbook_type        TEXT    NOT NULL,
    ticketbook_data        BLOB    NOT NULL UNIQUE,
    expiration_date        DATE    NOT NULL,
    epoch_id               INTEGER NOT NULL,
    total_tickets          INTEGER NOT NULL,
    used_tickets           INTEGER NOT NULL
);

INSERT INTO ecash_ticketbook_new (id, serialization_revision, ticketbook_type, ticketbook_data,
                                  expiration_date, epoch_id, total_tickets, used_tickets)
SELECT id,
       serialization_revision,
       ticketbook_type,
       ticketbook_data,
       expiration_date,
       epoch_id,
       total_tickets,
       used_tickets
FROM ecash_ticketbook;

DROP TABLE ecash_ticketbook;

ALTER TABLE ecash_ticketbook_new
    RENAME TO ecash_ticketbook;
