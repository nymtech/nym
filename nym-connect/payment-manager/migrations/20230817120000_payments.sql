CREATE TABLE payments
(
    id               INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    serial_number    VARCHAR NOT NULL UNIQUE,
    unyms_bought     INTEGER NOT NULL,
    paid             BOOLEAN NOT NULL
);