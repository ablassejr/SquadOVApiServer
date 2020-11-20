ALTER TABLE matches
DROP COLUMN parent_collection;

CREATE TABLE match_to_match_collection (
    match_uuid UUID NOT NULL REFERENCES matches(uuid) ON DELETE CASCADE,
    collection_uuid UUID NOT NULL REFERENCES match_collections(uuid) ON DELETE CASCADE,
    UNIQUE(collection_uuid, match_uuid)
);

CREATE INDEX ON match_to_match_collection(match_uuid);
CREATE INDEX ON match_to_match_collection(collection_uuid);