ALTER TABLE user_feature_flags
ADD COLUMN allow_separate_audio_channels BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE user_feature_flags
SET allow_separate_audio_channels = early_access;