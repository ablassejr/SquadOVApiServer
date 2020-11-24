DROP TABLE hearthstone_actions CASCADE;

ALTER TABLE hearthstone_raw_power_logs
DROP COLUMN raw_logs;

ALTER TABLE hearthstone_raw_power_logs
ADD COLUMN raw_logs_blob_uuid UUID NOT NULL REFERENCES blob_link_storage(uuid) ON DELETE CASCADE;

ALTER TABLE hearthstone_matches
ADD COLUMN actions_blob_uuid UUID NOT NULL REFERENCES blob_link_storage(uuid) ON DELETE CASCADE;