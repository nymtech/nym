-- IMPORTANT : At the time of writing this, there are no instances of the Stats API with data in that table. Dropping it to modify is therefore fine
DROP TABLE report_v2;

CREATE TABLE report_v2 (
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
