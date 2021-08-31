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

    INSERT INTO user_profile_vods (
        user_id,
        video_uuid
    )
    SELECT u.id, v.video_uuid
    FROM vods AS v
    INNER JOIN users AS u
        ON u.uuid = v.user_uuid
    WHERE v.end_time IS NOT NULL AND v.is_local = FALSE
        AND u.id = input_user_id;
END;
$$ LANGUAGE plpgsql;