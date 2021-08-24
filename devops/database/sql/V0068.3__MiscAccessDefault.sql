UPDATE user_profiles
SET misc_access = 1;

ALTER TABLE user_profiles
ALTER COLUMN misc_access SET NOT NULL;