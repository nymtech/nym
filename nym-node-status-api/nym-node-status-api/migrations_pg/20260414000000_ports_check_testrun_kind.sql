ALTER TABLE gateways
    ADD COLUMN last_ports_check_utc BIGINT;

ALTER TABLE testruns
    ADD COLUMN kind SMALLINT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_testruns_kind_status_created
    ON testruns (kind, status, created_utc);
