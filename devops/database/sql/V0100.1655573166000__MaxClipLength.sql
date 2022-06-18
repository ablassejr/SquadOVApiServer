ALTER TABLE user_feature_flags
ADD COLUMN max_clip_seconds BIGINT NOT NULL DEFAULT 120;