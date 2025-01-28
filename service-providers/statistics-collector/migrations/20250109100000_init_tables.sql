CREATE TABLE report (
    day          DATE NOT NULL,
    client_id    TEXT NOT NULL,
    client_type  TEXT,
    os_type      TEXT,
    os_version   TEXT,
    architecture TEXT,
    PRIMARY KEY (client_id, day)
); 

CREATE TABLE connection_stats (
    received_at           TIMESTAMP WITH TIME ZONE NOT NULL,
    client_id             TEXT                     NOT NULL,
    mixnet_entry_spent    INTEGER,
    vpn_entry_spent       INTEGER,
    mixnet_exit_spent     INTEGER,
    vpn_exit_spent        INTEGER,
    wg_exit_country_code  TEXT,
    mix_exit_country_code TEXT,
    PRIMARY KEY (client_id, received_at)
);


