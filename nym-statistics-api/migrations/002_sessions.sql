CREATE TABLE sessions_stats (
    received_at           TIMESTAMP WITH TIME ZONE  NOT NULL,
    day                   DATE,
    connection_time_ms    INTEGER,
    session_duration_min  INTEGER,
    two_hop               BOOLEAN,
    exit_id               TEXT,
    error                 TEXT,
    country_code          TEXT,
    from_mixnet           BOOLEAN
);