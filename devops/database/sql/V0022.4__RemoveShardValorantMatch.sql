ALTER TABLE valorant_match_uuid_link
DROP COLUMN shard CASCADE,
ADD CONSTRAINT valorant_match_uuid_link_match_id_key UNIQUE (match_id);