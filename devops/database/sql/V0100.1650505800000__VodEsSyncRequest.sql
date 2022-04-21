ALTER TABLE vods
ADD COLUMN request_sync_elasticsearch TIMESTAMPTZ;

UPDATE vods
SET request_sync_elasticsearch = last_sync_elasticsearch;