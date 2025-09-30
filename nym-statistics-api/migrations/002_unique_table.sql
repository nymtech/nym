
CREATE TABLE report_v1 (
    day                   DATE NOT NULL,
    received_at           TIMESTAMP WITH TIME ZONE  NOT NULL,
    source_ip             TEXT,
    device_id             TEXT NOT NULL,
    from_mixnet           BOOLEAN,
    os_type               TEXT,
    os_version            TEXT,
    architecture          TEXT,
    app_version           TEXT,
    user_agent            TEXT,
    connection_time_ms    INTEGER,
    two_hop               BOOLEAN,
    country_code          TEXT
); 

CREATE INDEX idx_report_v1_day ON report_v1 (day);