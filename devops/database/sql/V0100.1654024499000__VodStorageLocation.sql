DROP TABLE IF EXISTS vod_storage_copies;
CREATE TABLE vod_storage_copies (
    video_uuid UUID NOT NULL,
    -- 0: cloud, 1: local 
    loc INTEGER NOT NULL,
    spec VARCHAR NOT NULL,
    UNIQUE(video_uuid, loc, spec)
);

CREATE INDEX ON vod_storage_copies(loc, spec);

INSERT INTO vod_storage_copies (
    video_uuid,
    loc,
    spec
)
SELECT DISTINCT vm.video_uuid, 0, vm.bucket
FROM vod_metadata AS vm
INNER JOIN vods AS v
    ON vm.video_uuid = v.video_uuid
WHERE NOT v.is_local
ON CONFLICT DO NOTHING;