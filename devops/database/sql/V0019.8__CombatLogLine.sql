ALTER TABLE wow_combat_log_events
ADD COLUMN log_line BIGINT NOT NULL;

ALTER TABLE wow_combat_log_events
ADD CONSTRAINT wow_combat_log_events_combat_log_uuid_log_line_key UNIQUE(combat_log_uuid, log_line);