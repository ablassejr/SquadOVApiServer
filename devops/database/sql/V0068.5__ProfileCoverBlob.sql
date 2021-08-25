ALTER TABLE user_profiles
DROP COLUMN profile_picture_url,
DROP COLUMN cover_picture_url;

ALTER TABLE user_profiles
ADD COLUMN profile_picture_blob UUID REFERENCES blob_link_storage(uuid) ON DELETE SET NULL,
ADD COLUMN cover_picture_blob UUID REFERENCES blob_link_storage(uuid) ON DELETE SET NULL;