ALTER TABLE user_feature_flags
ALTER COLUMN watermark_min_size SET DEFAULT 0.05;

UPDATE user_feature_flags
SET watermark_min_size = 0.05;