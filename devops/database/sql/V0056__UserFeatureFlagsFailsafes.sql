ALTER TABLE user_feature_flags
DROP COLUMN enable_lol,
DROP COLUMN enable_tft,
ADD COLUMN max_record_pixel_y INTEGER NOT NULL DEFAULT 1080,
ADD COLUMN max_record_fps INTEGER NOT NULL DEFAULT 60,
ADD COLUMN allow_record_upload BOOLEAN NOT NULL DEFAULT TRUE;

INSERT INTO user_feature_flags (user_id)
SELECT id
FROM users
ON CONFLICT DO NOTHING;

CREATE OR REPLACE FUNCTION trigger_new_user_feature_flags()
    RETURNS trigger AS
$$
BEGIN
    INSERT INTO user_feature_flags (user_id)
    VALUES (NEW.id);

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS new_user_default_feature_flags ON users;
CREATE TRIGGER new_user_default_feature_flags
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION trigger_new_user_feature_flags();