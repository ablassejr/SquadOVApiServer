ALTER TABLE wow_combat_logs
ADD COLUMN blob_uuid UUID NOT NULL REFERENCES blob_link_storage(uuid) ON DELETE CASCADE;

ALTER TABLE blob_link_storage
ADD COLUMN session_uri VARCHAR;