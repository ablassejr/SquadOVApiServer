CREATE INDEX ON share_match_vod_connections(source_user_id, match_uuid)
WHERE match_uuid IS NOT NULL;

CREATE INDEX ON share_match_vod_connections(source_user_id, video_uuid)
WHERE video_uuid IS NOT NULL;