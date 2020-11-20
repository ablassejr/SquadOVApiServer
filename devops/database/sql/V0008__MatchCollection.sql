CREATE TABLE match_collections (
    uuid UUID PRIMARY KEY
);

ALTER TABLE matches
ADD COLUMN parent_collection UUID REFERENCES match_collections(uuid) ON DELETE CASCADE;