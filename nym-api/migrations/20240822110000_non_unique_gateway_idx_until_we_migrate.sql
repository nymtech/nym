DROP TABLE IF EXISTS gateway_details_v2;

CREATE TABLE gateway_details_v2
(
    id      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    node_id  INTEGER NOT NULL,
    identity VARCHAR NOT NULL UNIQUE
);


