CREATE SEQUENCE share_match_vod_connections_id_seq OWNED BY share_match_vod_connections.id;

ALTER TABLE share_match_vod_connections
ALTER COLUMN id SET DEFAULT nextval('share_match_vod_connections_id_seq');

SELECT setval('share_match_vod_connections_id_seq', COALESCE(MAX(id), 0)) FROM share_match_vod_connections;

UPDATE share_match_vod_connections
SET id = nextval('share_match_vod_connections_id_seq')
WHERE id IS NULL;

ALTER TABLE share_match_vod_connections
ADD PRIMARY KEY (id);

ALTER TABLE share_match_vod_connections
ALTER COLUMN can_share SET NOT NULL,
ALTER COLUMN can_clip SET NOT NULL;