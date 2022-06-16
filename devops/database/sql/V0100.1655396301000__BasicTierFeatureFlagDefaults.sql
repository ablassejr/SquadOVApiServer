ALTER TABLE user_feature_flags
ALTER COLUMN max_record_pixel_y SET DEFAULT 720,
ALTER COLUMN max_record_fps SET DEFAULT 60,
ALTER COLUMN max_bitrate_kbps SET DEFAULT 6000,
ALTER COLUMN mandatory_watermark SET DEFAULT TRUE,
ALTER COLUMN watermark_min_size SET DEFAULT 0.1,
ALTER COLUMN vod_priority SET DEFAULT 5,
ALTER COLUMN early_access SET DEFAULT FALSE,
ALTER COLUMN vod_retention SET DEFAULT 604800,
ALTER COLUMN max_squad_size SET DEFAULT 20;

UPDATE user_feature_flags
SET max_record_pixel_y = default,
    max_record_fps = default,
    max_bitrate_kbps = default,
    mandatory_watermark = default,
    watermark_min_size = default,
    vod_priority = default,
    early_access = default,
    vod_retention = default,
    max_squad_size = default;