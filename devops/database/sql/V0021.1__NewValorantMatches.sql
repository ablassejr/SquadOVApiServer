DELETE FROM matches
WHERE uuid IN (
    SELECT match_uuid
    FROM valorant_matches
);

ALTER TABLE valorant_matches
RENAME COLUMN map TO map_id;

ALTER TABLE valorant_matches
ADD COLUMN game_length_millis INTEGER NOT NULL,
ADD COLUMN season_id VARCHAR,
DROP COLUMN game_version;