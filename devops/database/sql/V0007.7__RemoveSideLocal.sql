ALTER TABLE hearthstone_match_players
DROP COLUMN side;

ALTER TABLE hearthstone_match_players
DROP COLUMN is_local;

ALTER TABLE hearthstone_raw_power_logs
ADD CONSTRAINT hearthstone_raw_power_logs_match_uuid_user_id_unique UNIQUE(match_uuid, user_id);