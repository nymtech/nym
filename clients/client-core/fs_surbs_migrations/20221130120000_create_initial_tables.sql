CREATE TABLE status
(
    flush_in_progress        INTEGER NOT NULL,
    previous_flush_timestamp INTEGER NOT NULL,
    client_in_use            INTEGER NOT NULL
);

CREATE TABLE reply_surb_storage_metadata
(
    min_reply_surb_threshold INTEGER NOT NULL,
    max_reply_surb_threshold INTEGER NOT NULL
);

CREATE TABLE sender_tag
(
    recipient BLOB NOT NULL UNIQUE,
    tag       BLOB NOT NULL UNIQUE
);

CREATE TABLE reply_key
(
    key_digest        BLOB    NOT NULL UNIQUE,
    reply_key         BLOB    NOT NULL UNIQUE,
    sent_at_timestamp INTEGER NOT NULL
);

CREATE TABLE reply_surb_sender
(
    id                  INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    last_sent_timestamp INTEGER NOT NULL,
    tag                 BLOB    NOT NULL UNIQUE
);

CREATE TABLE reply_surb
(
    reply_surb_sender_id INTEGER NOT NULL,
    reply_surb           BLOB    NOT NULL,

    FOREIGN KEY (reply_surb_sender_id) REFERENCES reply_surb_sender (id)
);