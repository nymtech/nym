/*
 * Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

-- change `previous_flush_timestamp` unix timestamp to `previous_flush` timestamp
CREATE TABLE status_new
(
    flush_in_progress INTEGER                     NOT NULL,
    previous_flush    TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    client_in_use     INTEGER                     NOT NULL
);

INSERT INTO status_new (flush_in_progress, previous_flush, client_in_use)
SELECT flush_in_progress,
       datetime(previous_flush_timestamp, 'unixepoch') AS previous_flush,
       client_in_use
FROM status;

DROP TABLE status;
ALTER TABLE status_new
    RENAME TO status;


-- change `sent_at_timestamp` unix timestamp to `sent_at` timestamp
CREATE TABLE reply_key_new
(
    key_digest BLOB                        NOT NULL UNIQUE,
    reply_key  BLOB                        NOT NULL UNIQUE,
    sent_at    TIMESTAMP WITHOUT TIME ZONE NOT NULL
);

INSERT INTO reply_key_new (key_digest, reply_key, sent_at)
SELECT key_digest,
       reply_key,
       datetime(sent_at_timestamp, 'unixepoch') AS sent_at
FROM reply_key;

DROP TABLE reply_key;
ALTER TABLE reply_key_new
    RENAME TO reply_key;


-- change `last_sent_timestamp` unix timestamp to `sent_at` last_sent
CREATE TABLE reply_surb_sender_new
(
    id        INTEGER                     NOT NULL PRIMARY KEY AUTOINCREMENT,
    last_sent TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    tag       BLOB                        NOT NULL UNIQUE
);

INSERT INTO reply_surb_sender_new (id, last_sent, tag)
SELECT id,
       datetime(last_sent_timestamp, 'unixepoch') AS last_sent,
       tag
FROM reply_surb_sender;

DROP TABLE reply_surb_sender;
ALTER TABLE reply_surb_sender_new
    RENAME TO reply_surb_sender;