ALTER TABLE tft_match_participants
ALTER COLUMN companion_skin_id TYPE INTEGER USING companion_skin_id::INTEGER;