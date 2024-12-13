ALTER TABLE testruns
RENAME COLUMN timestamp_utc TO created_utc;

ALTER TABLE testruns
ADD COLUMN last_assigned_utc INTEGER;
