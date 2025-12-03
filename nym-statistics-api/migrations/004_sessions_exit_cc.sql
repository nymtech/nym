CREATE TABLE report_v2_new (
    -- some info about the report, inferred from when/from where we got it
    received_at           TIMESTAMP WITH TIME ZONE  NOT NULL,
    source_ip             TEXT,
    from_mixnet           BOOLEAN,
    country_code          TEXT,
    report_version        TEXT,

    -- some infos about the device sending the report
    device_id             TEXT NOT NULL,
    os_type               TEXT,
    os_version            TEXT,
    architecture          TEXT,
    app_version           TEXT,
    user_agent            TEXT,

    -- session info
    start_day_utc         DATE,
    connection_time_ms    INTEGER,
    tunnel_type           TEXT,
    retry_attempt         INTEGER,
    session_duration_min  INTEGER,
    disconnection_time_ms INTEGER,
    exit_id               TEXT,
    exit_cc               TEXT,   -- new column
    follow_up_id          TEXT,
    error                 TEXT
);

--  copy data over
INSERT INTO report_v2_new (
    received_at, source_ip, from_mixnet, country_code, report_version,
    device_id, os_type, os_version, architecture, app_version, user_agent,
    start_day_utc, connection_time_ms, tunnel_type, retry_attempt,
    session_duration_min, disconnection_time_ms,
    exit_id, exit_cc, follow_up_id, error
)
SELECT
    received_at, source_ip, from_mixnet, country_code, report_version,
    device_id, os_type, os_version, architecture, app_version, user_agent,
    start_day_utc, connection_time_ms, tunnel_type, retry_attempt,
    session_duration_min, disconnection_time_ms,
    exit_id, NULL AS exit_cc, follow_up_id, error
FROM report_v2;

-- Drop old table and rename
DROP TABLE report_v2;
ALTER TABLE report_v2_new RENAME TO report_v2;
CREATE INDEX idx_report_v2_received_at ON report_v2 (received_at);
