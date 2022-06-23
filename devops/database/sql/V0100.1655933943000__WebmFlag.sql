ALTER TABLE user_feature_flags
ADD COLUMN allow_vp9 BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE user_feature_flags
SET allow_vp9 = early_access;