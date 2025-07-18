CREATE TABLE mixnode_packet_stats_raw (
    id SERIAL PRIMARY KEY,
    mix_id BIGINT NOT NULL,
    timestamp_utc BIGINT NOT NULL,
    packets_received INTEGER,
    packets_sent INTEGER,
    packets_dropped INTEGER,
    FOREIGN KEY (mix_id) REFERENCES mixnodes (mix_id)
);

CREATE INDEX idx_mixnode_packet_stats_raw_mix_id_timestamp_utc ON mixnode_packet_stats_raw (mix_id, timestamp_utc);