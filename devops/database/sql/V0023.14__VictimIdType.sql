ALTER TABLE lol_match_timeline_events
ALTER COLUMN victim_id TYPE INTEGER USING victim_id::INTEGER;