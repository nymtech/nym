-- Re-run ports_check extraction for rows missed by the initial migration
-- (e.g. if it failed partway through on invalid JSON rows before the guard was added).
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
