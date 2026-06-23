ALTER TABLE gateways ADD COLUMN IF NOT EXISTS ports_check JSONB;

-- Some gateways store probe payloads that start with '{'/'[' but are not valid JSON
-- (truncated logs, legacy text, etc.). Casting those rows fails the whole migration,
-- so process row-by-row and skip invalid entries.
DO $$
DECLARE
    r RECORD;
    probe_json JSONB;
BEGIN
    FOR r IN
        SELECT id, last_probe_result
        FROM gateways
        WHERE last_probe_result IS NOT NULL
          AND btrim(last_probe_result) <> ''
          AND last_probe_result ~ '^[\[{]'
          AND ports_check IS NULL
    LOOP
        BEGIN
            probe_json := r.last_probe_result::jsonb;

            IF probe_json ? 'ports_check' THEN
                UPDATE gateways
                SET ports_check = probe_json -> 'ports_check'
                WHERE id = r.id;

                UPDATE gateways
                SET last_probe_result = (probe_json - 'ports_check')::text
                WHERE id = r.id;
            END IF;
        EXCEPTION
            WHEN invalid_text_representation THEN
                NULL;
        END;
    END LOOP;
END $$;
