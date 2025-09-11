-- for a couple of days after migrating chrono -> time, we stored dates as
-- 2025-June-DD instead of 2025-06-DD. This migration fixes those entries.
--
-- Because of a UNIQUE constraint on (node_id, date_utc), we can't just rename in-place.
--   - merge (add) node stats back to the original table where conflict (node_id, date_utc) would exist
--   - delete invalid records from original table (those stats were merged into correct rows above)
--   - insert rows that did not have a conflicting (node_Id, date_utc) combo
-- Conflicts affect only the date which has both kinds of entries,
-- e.g. 2025-06-05 and 2025-June-05 (date when this change was deployed)
--
-- This applies to both affected tables.

-- ----------------------------------------
-- mixnode_daily_stats
-- ----------------------------------------

-- First, copy over rows with invalid date to a temp table (in the correct date format)
CREATE TEMP TABLE tmp_mix AS
SELECT
  mix_id,
  REPLACE(date_utc,'June','06') AS new_date,
  SUM(total_stake)      AS total_stake_sum,
  SUM(packets_received) AS packets_received_sum,
  SUM(packets_sent)     AS packets_sent_sum,
  SUM(packets_dropped)  AS packets_dropped_sum
FROM mixnode_daily_stats
WHERE date_utc LIKE '%June%'
GROUP BY mix_id, new_date;

UPDATE mixnode_daily_stats AS m
SET
    total_stake      = m.total_stake,
    packets_received = m.packets_received + (SELECT packets_received_sum FROM tmp_mix WHERE mix_id = m.mix_id AND new_date = m.date_utc),
    packets_sent     = m.packets_sent     + (SELECT packets_sent_sum     FROM tmp_mix WHERE mix_id = m.mix_id AND new_date = m.date_utc),
    packets_dropped  = m.packets_dropped  + (SELECT packets_dropped_sum  FROM tmp_mix WHERE mix_id = m.mix_id AND new_date = m.date_utc)
WHERE EXISTS (
  SELECT 1 FROM tmp_mix
   WHERE mix_id   = m.mix_id
     AND new_date = m.date_utc
);

DELETE FROM mixnode_daily_stats
 WHERE date_utc LIKE '%June%';

INSERT INTO mixnode_daily_stats
   (mix_id, date_utc, total_stake, packets_received, packets_sent, packets_dropped)
SELECT
  mix_id,
  new_date,
  total_stake_sum,
  packets_received_sum,
  packets_sent_sum,
  packets_dropped_sum
FROM tmp_mix AS t
-- only those whose new_date did _not_ already exist
WHERE NOT EXISTS (
  SELECT 1 FROM mixnode_daily_stats AS m
   WHERE m.mix_id   = t.mix_id
     AND m.date_utc = t.new_date
);

DROP TABLE tmp_mix;


-- ----------------------------------------
-- nym_node_daily_mixing_stats
-- ----------------------------------------

CREATE TEMP TABLE tmp_nym_node_stats AS
SELECT
  node_id,
  REPLACE(date_utc,'June','06') AS new_date,
  SUM(total_stake)      AS total_stake_sum,
  SUM(packets_received) AS packets_received_sum,
  SUM(packets_sent)     AS packets_sent_sum,
  SUM(packets_dropped)  AS packets_dropped_sum
FROM nym_node_daily_mixing_stats
WHERE date_utc LIKE '%June%'
GROUP BY node_id, new_date;

UPDATE nym_node_daily_mixing_stats AS m
SET
    total_stake      = m.total_stake,
    packets_received = m.packets_received + (SELECT packets_received_sum FROM tmp_nym_node_stats WHERE node_id = m.node_id AND new_date = m.date_utc),
    packets_sent     = m.packets_sent     + (SELECT packets_sent_sum     FROM tmp_nym_node_stats WHERE node_id = m.node_id AND new_date = m.date_utc),
    packets_dropped  = m.packets_dropped  + (SELECT packets_dropped_sum  FROM tmp_nym_node_stats WHERE node_id = m.node_id AND new_date = m.date_utc)
WHERE EXISTS (
  SELECT 1 FROM tmp_nym_node_stats
   WHERE node_id  = m.node_id
     AND new_date = m.date_utc
);

DELETE FROM nym_node_daily_mixing_stats
 WHERE date_utc LIKE '%June%';

INSERT INTO nym_node_daily_mixing_stats
   (node_id, date_utc, total_stake, packets_received, packets_sent, packets_dropped)
SELECT
  node_id,
  new_date,
  total_stake_sum,
  packets_received_sum,
  packets_sent_sum,
  packets_dropped_sum
FROM tmp_nym_node_stats AS t
WHERE NOT EXISTS (
  SELECT 1 FROM nym_node_daily_mixing_stats AS m
   WHERE m.node_id  = t.node_id
     AND m.date_utc = t.new_date
);

DROP TABLE tmp_nym_node_stats;

