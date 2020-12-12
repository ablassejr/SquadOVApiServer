-- We need this constraint because we need to be able to identify the cases where
-- 1) The user inserts an existing valorant match ID with a new match uuid
-- 2) The user inserts an existing valorant match ID with the existing match uuid.
-- Case 1 is an error while case 2 is not.
ALTER TABLE valorant_matches
ADD CONSTRAINT valorant_matches_match_id_match_uuid_key UNIQUE(match_id, match_uuid);