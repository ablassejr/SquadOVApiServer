ALTER TABLE tft_matches
ADD COLUMN game_start_time TIMESTAMPTZ;

ALTER TABLE lol_matches
ADD COLUMN game_start_time TIMESTAMPTZ;