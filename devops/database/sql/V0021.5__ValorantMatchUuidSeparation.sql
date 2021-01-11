ALTER TABLE valorant_matches
DROP COLUMN match_uuid;

CREATE TABLE valorant_match_uuid_link (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    match_id VARCHAR NOT NULL UNIQUE
);

ALTER TABLE valorant_matches
ADD CONSTRAINT valorant_matches_match_id_fkey FOREIGN KEY (match_id) REFERENCES valorant_match_uuid_link (match_id) ON DELETE CASCADE;