ALTER TABLE lol_match_participants
ALTER COLUMN first_blood_kill TYPE BOOLEAN USING first_blood_kill::BOOLEAN;