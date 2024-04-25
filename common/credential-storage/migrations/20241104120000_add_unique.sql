/*
 * Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
 * SPDX-License-Identifier: Apache-2.0
 */

ALTER TABLE coconut_credentials
    RENAME TO old_coconut_credentials;

CREATE TABLE coconut_credentials
(
    id                     INTEGER                                                                     NOT NULL PRIMARY KEY AUTOINCREMENT,

--     introduce a way for us to introduce breaking changes in serialization
    serialization_revision INTEGER                                                                     NOT NULL,

--     the best we can do without enums
    credential_type        TEXT CHECK ( credential_type IN ('BandwidthVoucher', 'FreeBandwidthPass') ) NOT NULL,
    credential_data        BLOB                                                                        NOT NULL UNIQUE,
    epoch_id               INTEGER                                                                     NOT NULL,

--     this field is only really applicable to free passes
    expired                BOOLEAN                                                                     NOT NULL
);

ALTER TABLE credential_usage
    RENAME TO old_credential_usage;

-- for bandwidth vouchers there's going to be only a single entry; for freepasses there can be as many as there are gateways
CREATE TABLE credential_usage
(
    credential_id   INTEGER NOT NULL REFERENCES coconut_credentials (id),
    gateway_id_bs58 TEXT    NOT NULL,

--     no matter credential type, we can't spend the same credential with the same gateway multiple times
    UNIQUE (credential_id, gateway_id_bs58)
);

INSERT INTO coconut_credentials
SELECT *
FROM old_coconut_credentials;


INSERT INTO credential_usage
SELECT *
FROM old_credential_usage;

DROP TABLE old_coconut_credentials;
DROP TABLE old_credential_usage;