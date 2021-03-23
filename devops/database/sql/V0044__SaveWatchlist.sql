CREATE TABLE user_favorite_matches (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    reason VARCHAR NOT NULL,
    PRIMARY KEY(user_id, match_uuid)
);

CREATE TABLE user_favorite_vods (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    video_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    reason VARCHAR NOT NULL,
    PRIMARY KEY(user_id, video_uuid)
);

CREATE TABLE user_watchlist_vods (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    video_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    PRIMARY KEY(user_id, video_uuid)
);