CREATE TABLE vod_clips (
    clip_uuid UUID UNIQUE NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    parent_vod_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    clip_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR NOT NULL,
    description VARCHAR NOT NULL
);

CREATE INDEX ON vod_clips(parent_vod_uuid);
CREATE INDEX ON vod_clips(clip_user_id);