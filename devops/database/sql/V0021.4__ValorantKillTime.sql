ALTER TABLE valorant_match_kill
DROP COLUMN round_time,
ADD COLUMN time_since_game_start_millis INTEGER NOT NULL,
ADD COLUMN time_since_round_start_millis INTEGER NOT NULL,
ADD COLUMN assistants VARCHAR[];