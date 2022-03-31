ALTER TABLE match_custom_events
DROP COLUMN event_id CASCADE;

ALTER TABLE match_custom_events
ADD COLUMN event_id BIGSERIAL PRIMARY KEY;