CREATE INDEX CONCURRENTLY ON user_watchlist_vods(video_uuid);
CREATE INDEX CONCURRENTLY ON user_favorite_matches(match_uuid);
CREATE INDEX CONCURRENTLY ON user_favorite_vods(video_uuid);
CREATE INDEX CONCURRENTLY ON user_profile_vods(video_uuid);