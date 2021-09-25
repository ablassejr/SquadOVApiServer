ALTER TABLE user_profile_vods
ADD CONSTRAINT user_profile_vods_user_id_video_uuid_key UNIQUE(user_id, video_uuid);

CREATE UNIQUE INDEX ON squadov.user_autosharing_settings(source_user_id)
WHERE dest_user_id IS NULL AND dest_squad_id IS NULL;

CREATE OR REPLACE FUNCTION fix_autoshare_to_profile(input_user_id BIGINT)
    RETURNS VOID AS
$$
DECLARE
    new_share_id BIGINT;
BEGIN
    INSERT INTO squadov.user_autosharing_settings (
        source_user_id,
        can_share,
        can_clip
    ) VALUES (
        input_user_id,
        TRUE,
        TRUE
    )
    ON CONFLICT DO NOTHING;

    SELECT id INTO new_share_id
    FROM squadov.user_autosharing_settings
    WHERE source_user_id = input_user_id
        AND dest_user_id IS NULL
        AND dest_squad_id IS NULL;
    
    INSERT INTO squadov.user_autosharing_settings_games (
        id,
        game
    )
    SELECT uas.id, g.game
    FROM squadov.user_autosharing_settings AS uas
    CROSS JOIN UNNEST(ARRAY[0,1,2,3,4,5,6]) AS g(game)
    WHERE uas.id = new_share_id
    ON CONFLICT DO NOTHING;

    INSERT INTO squadov.user_profile_vods (
        user_id,
        video_uuid
    )
    SELECT u.id, v.video_uuid
    FROM squadov.vods AS v
    INNER JOIN squadov.users AS u
        ON u.uuid = v.user_uuid
    WHERE v.end_time IS NOT NULL AND v.is_local = FALSE
        AND u.id = input_user_id
    ON CONFLICT DO NOTHING;
END;
$$ LANGUAGE plpgsql;

DO $$ BEGIN
    PERFORM fix_autoshare_to_profile(user_id)
    FROM user_profiles;
END $$ LANGUAGE plpgsql;

DROP FUNCTION IF EXISTS fix_autoshare_to_profile;