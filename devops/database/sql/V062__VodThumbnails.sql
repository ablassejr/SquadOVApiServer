CREATE TABLE vod_thumbnails (
    video_uuid UUID UNIQUE NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE,
    bucket VARCHAR NOT NULL,
    filepath VARCHAR NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    is_public BOOLEAN NOT NULL DEFAULT FALSE
);