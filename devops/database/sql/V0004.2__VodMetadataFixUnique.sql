ALTER TABLE vod_metadata
DROP CONSTRAINT vod_metadata_video_uuid_id_key CASCADE;

ALTER TABLE vod_metadata
ADD CONSTRAINT vod_metadata_video_uuid_id_data_type_key UNIQUE(video_uuid, data_type, id);