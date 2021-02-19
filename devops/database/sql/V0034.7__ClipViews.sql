CREATE TABLE clip_views (
    clip_uuid UUID NOT NULL REFERENCES vod_clips(clip_uuid) ON DELETE CASCADE,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON clip_views(clip_uuid);
CREATE INDEX ON clip_views(user_id);

CREATE VIEW view_clip_view_count (
    clip_uuid,
    count
)
AS
SELECT
    cv.clip_uuid,
    COUNT(cv.user_id)
FROM clip_views AS cv
GROUP BY cv.clip_uuid;