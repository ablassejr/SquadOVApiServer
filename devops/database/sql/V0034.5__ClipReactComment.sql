CREATE TABLE clip_reacts (
    clip_uuid UUID NOT NULL REFERENCES vod_clips(clip_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL,
    UNIQUE(clip_uuid, user_id)
);

CREATE TABLE clip_comments (
    clip_uuid UUID NOT NULL REFERENCES vod_clips(clip_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment VARCHAR NOT NULL,
    tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON clip_comments(clip_uuid, user_id);