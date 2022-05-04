ALTER TABLE user_event_record
ADD COLUMN platform VARCHAR;

ALTER TABLE user_event_record
DROP CONSTRAINT user_event_record_user_id_event_name_key;

ALTER TABLE user_event_record
ADD CONSTRAINT user_event_record_user_id_event_name_platform_key UNIQUE(user_id, event_name, platform);