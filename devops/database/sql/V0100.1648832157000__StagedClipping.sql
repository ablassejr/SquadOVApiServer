CREATE TABLE staged_clips (
    id BIGSERIAL PRIMARY KEY,
    video_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    start_offset_ms BIGINT NOT NULL,
    end_offset_ms BIGINT NOT NULL
);

CREATE INDEX ON staged_clips(video_uuid, user_id);