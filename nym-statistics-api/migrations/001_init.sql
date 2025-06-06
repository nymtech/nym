
CREATE TABLE active_device (
    day          DATE NOT NULL,
    device_id    TEXT NOT NULL,
    os_type      TEXT,
    os_version   TEXT,
    architecture TEXT,
    app_version  TEXT,
    user_agent   TEXT,
    from_mixnet  BOOLEAN,
    PRIMARY KEY (device_id, day)
); 

CREATE TABLE connection_stats (
    received_at           TIMESTAMP WITH TIME ZONE  NOT NULL,
    connection_time_ms    INTEGER,
    two_hop               BOOLEAN,
    source_ip             TEXT,
    country_code          TEXT,
    from_mixnet           BOOLEAN
);


CREATE INDEX idx_active_device_day ON active_device (day);