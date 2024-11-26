
CREATE TABLE gateway_session_stats (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    gateway_identity_key  VARCHAR NOT NULL,
    node_id               INTEGER NOT NULL,
    date_utc              DATE    NOT NULL,
    unique_active_clients INTEGER NOT NULL,
    session_started       INTEGER NOT NULL,
    users_hashes          VARCHAR,
    vpn_sessions          VARCHAR,
    mixnet_sessions       VARCHAR,
    unknown_sessions      VARCHAR,
    UNIQUE (node_id, date_utc) -- This constraint automatically creates an index
  );
CREATE INDEX idx_gateway_session_stats_identity_key ON gateway_session_stats (gateway_identity_key);

