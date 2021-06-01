CREATE INDEX ON share_match_vod_connections(match_uuid, dest_user_id)
WHERE match_uuid IS NOT NULL AND dest_user_id IS NOT NULL;

CREATE INDEX ON share_match_vod_connections(match_uuid, dest_squad_id)
WHERE match_uuid IS NOT NULL AND dest_squad_id IS NOT NULL;

CREATE INDEX ON share_match_vod_connections(video_uuid, dest_user_id)
WHERE video_uuid IS NOT NULL AND dest_user_id IS NOT NULL;

CREATE INDEX ON share_match_vod_connections(video_uuid, dest_squad_id)
WHERE video_uuid IS NOT NULL AND dest_squad_id IS NOT NULL;