ALTER TABLE staged_clips
ADD COLUMN clip_uuid UUID REFERENCES vod_clips(clip_uuid) ON DELETE CASCADE;