ALTER TABLE vods
DROP CONSTRAINT vods_user_uuid_fkey,
ADD CONSTRAINT vods_user_uuid_fkey FOREIGN KEY (user_uuid) REFERENCES users(uuid) ON DELETE SET NULL;