ALTER TABLE vod_clips
ALTER COLUMN parent_vod_uuid DROP NOT NULL;

ALTER TABLE vod_clips
DROP CONSTRAINT vod_clips_parent_vod_uuid_fkey,
ADD CONSTRAINT vod_clips_parent_vod_uuid_fkey FOREIGN KEY(parent_vod_uuid) REFERENCES vods(video_uuid) ON DELETE SET NULL;