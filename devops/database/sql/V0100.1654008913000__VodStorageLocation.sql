CREATE TABLE vod_storage_copies (
    video_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    -- 0: cloud, 1: local 
    loc INTEGER NOT NULL,
    spec VARCHAR NOT NULL
);

CREATE INDEX ON vod_storage_copies(video_uuid, loc, spec);

INSERT INTO vod_storage_copies (
    video_uuid,
    loc,
    spec
)
SELECT video_uuid, 0, bucket
FROM vod_metadata;