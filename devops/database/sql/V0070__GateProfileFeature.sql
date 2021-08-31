DROP TRIGGER IF EXISTS trigger_create_user_profile ON users;
DROP FUNCTION IF EXISTS fn_trigger_create_user_profile;

DELETE FROM user_profiles;
DELETE FROM user_profile_vods;
DELETE FROM user_autosharing_settings
WHERE dest_user_id IS NULL AND dest_squad_id IS NULL;

DROP FUNCTION IF EXISTS create_user_profile;
CREATE OR REPLACE FUNCTION create_user_profile(input_user_id BIGINT, input_slug VARCHAR)
    RETURNS VOID AS
$$
DECLARE
    new_share_id BIGINT;
BEGIN
    INSERT INTO squadov.user_profiles (
        user_id,
        link_slug,
        display_name,
        description,
        achievement_access,
        match_access
    )
    SELECT id, input_slug, username, '', 1, 1
    FROM squadov.users
    WHERE id = input_user_id;

    INSERT INTO squadov.user_autosharing_settings (
        source_user_id,
        can_share,
        can_clip
    ) VALUES (
        input_user_id,
        TRUE,
        TRUE
    ) RETURNING id INTO new_share_id;

    INSERT INTO user_autosharing_settings_games (
        id,
        game
    )
    SELECT uas.id, g.game
    FROM user_autosharing_settings AS uas
    CROSS JOIN UNNEST(ARRAY[0,1,2,3,4,5,6]) AS g(game)
    WHERE uas.id = new_share_id
    ON CONFLICT DO NOTHING;
END;
$$ LANGUAGE plpgsql;

ALTER TABLE user_feature_flags
ADD COLUMN enable_user_profiles BOOLEAN NOT NULL DEFAULT FALSE;