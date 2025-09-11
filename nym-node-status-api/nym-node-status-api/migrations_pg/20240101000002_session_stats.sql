CREATE TABLE gateway_session_stats (
    id                    SERIAL PRIMARY KEY,
    gateway_identity_key  VARCHAR NOT NULL,
    node_id               BIGINT NOT NULL,
    day                   DATE    NOT NULL,
    unique_active_clients BIGINT NOT NULL,
    session_started       BIGINT NOT NULL,
    users_hashes          VARCHAR,
    vpn_sessions          VARCHAR,
    mixnet_sessions       VARCHAR,
    unknown_sessions      VARCHAR,
    UNIQUE (node_id, day) -- This constraint automatically creates an index
);

CREATE INDEX idx_gateway_session_stats_identity_key ON gateway_session_stats (gateway_identity_key);
CREATE INDEX idx_gateway_session_stats_day ON gateway_session_stats (day);