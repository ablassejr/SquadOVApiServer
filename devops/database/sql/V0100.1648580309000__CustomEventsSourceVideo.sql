ALTER TABLE match_custom_events
ALTER COLUMN match_uuid DROP NOT NULL,
ADD COLUMN video_uuid UUID REFERENCES vods(video_uuid) ON DELETE SET NULL;

CREATE INDEX ON match_custom_events(video_uuid);