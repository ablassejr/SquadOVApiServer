CREATE TABLE vods (
    match_uuid UUID REFERENCES matches (uuid) ON DELETE CASCADE,
    user_uuid UUID REFERENCES users(uuid) ON DELETE CASCADE,
    video_uuid UUID NOT NULL,
    start_time TIMESTAMPTZ,
    end_time TIMESTAMPTZ
);