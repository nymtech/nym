CREATE TABLE report_v2 (
    -- some info about the report, inferred from when/from where we got it
    received_at           TIMESTAMP WITH TIME ZONE  NOT NULL,
    source_ip             TEXT,
    from_mixnet           BOOLEAN,
    country_code          TEXT,

    -- some infos about the device sending the report
    device_id             TEXT NOT NULL,
    os_type               TEXT,
    os_version            TEXT,
    architecture          TEXT,
    app_version           TEXT,
    user_agent            TEXT,

    -- session info
    start_day             DATE,
    connection_time_ms    INTEGER,
    two_hop               BOOLEAN,
    session_duration_min  INTEGER,
    exit_id               TEXT,
    error                 TEXT
); 

CREATE INDEX idx_report_v2_received_at ON report_v2 (received_at);
