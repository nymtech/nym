CREATE TABLE presence_announcements
(
    id INTEGER PRIMARY KEY NOT NULL,
    host VARCHAR NOT NULL,
    public_key VARCHAR NOT NULL,
    node_type VARCHAR NOT NULL
)