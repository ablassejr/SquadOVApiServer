CREATE TABLE vod_metadata (
    video_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    res_x INTEGER NOT NULL,
    res_y INTEGER NOT NULL,
    min_bitrate BIGINT NOT NULL,
    avg_bitrate BIGINT NOT NULL,
    max_bitrate BIGINT NOT NULL,
    fps INTEGER NOT NULL,
    id VARCHAR NOT NULL,
    data_type VARCHAR NOT NULL,
    UNIQUE(video_uuid, id)
);