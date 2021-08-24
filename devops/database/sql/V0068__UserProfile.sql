CREATE TABLE user_profiles (
    user_id BIGINT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    link_slug VARCHAR NOT NULL UNIQUE,
    display_name VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    /* Bitfields: 0: Self, 1: Private (Squads), 2: Private (Twitch sub), 4: Private (SquadOV sub), 8: Public */
    achievement_access INTEGER NOT NULL,
    match_access INTEGER NOT NULL,
    profile_picture_url VARCHAR,
    cover_picture_url VARCHAR
);

CREATE TABLE user_profile_achievements(
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    game INTEGER NOT NULL,
    subtype VARCHAR,
    tm TIMESTAMPTZ NOT NULL,
    data JSONB NOT NULL,
    UNIQUE(user_id, game, subtype)
);

/* VODs and Clips! */
CREATE TABLE user_profile_vods (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    video_uuid UUID NOT NULL REFERENCES vods(video_uuid) ON DELETE CASCADE
);

/* Auto share */
CREATE OR REPLACE FUNCTION fn_trigger_auto_share()
    RETURNS trigger AS
$$
DECLARE
    vod_user_id BIGINT;
    conn_match_uuid UUID;
    conn_video_uuid UUID;
BEGIN
    IF NEW.match_uuid IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT id INTO vod_user_id
    FROM squadov.users
    WHERE uuid = NEW.user_uuid;

    IF NEW.is_clip THEN
        conn_match_uuid := NULL;
        conn_video_uuid := NEW.video_uuid;
    ELSE
        conn_match_uuid := NEW.match_uuid;
        conn_video_uuid := NEW.video_uuid;
    END IF;

    INSERT INTO squadov.share_match_vod_connections (
        match_uuid,
        video_uuid,
        source_user_id,
        dest_user_id,
        dest_squad_id,
        can_share,
        can_clip,
        parent_connection_id,
        share_depth
    )
    SELECT
        conn_match_uuid,
        conn_video_uuid,
        vod_user_id,
        uas.dest_user_id,
        uas.dest_squad_id,
        uas.can_share,
        uas.can_clip,
        NULL,
        0
    FROM squadov.user_autosharing_settings AS uas
    CROSS JOIN (
        SELECT *
        FROM squadov.matches AS m
        WHERE m.uuid = NEW.match_uuid
    ) AS m
    LEFT JOIN squadov.user_autosharing_settings_games AS asg
        ON asg.id = uas.id
            AND asg.game = m.game
    WHERE uas.source_user_id = vod_user_id
        AND (uas.dest_user_id IS NOT NULL OR uas.dest_squad_id IS NOT NULL)
        AND asg.id IS NOT NULL;
    
    INSERT INTO squadov.user_profile_vods (
        user_id,
        video_uuid
    )
    SELECT
        vod_user_id,
        conn_video_uuid
    FROM squadov.user_autosharing_settings AS uas
    CROSS JOIN (
        SELECT *
        FROM squadov.matches AS m
        WHERE m.uuid = NEW.match_uuid
    ) AS m
    LEFT JOIN squadov.user_autosharing_settings_games AS asg
        ON asg.id = uas.id
            AND asg.game = m.game
    WHERE uas.source_user_id = vod_user_id
        AND uas.dest_user_id IS NULL
        AND uas.dest_squad_id IS NULL
        AND asg.id IS NOT NULL;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION create_user_profile(input_user_id BIGINT, input_username VARCHAR)
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
    ) VALUES (
        input_user_id,
        input_username,
        input_username,
        '',
        1,
        1
    );

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

CREATE OR REPLACE FUNCTION fn_trigger_create_user_profile()
    RETURNS trigger AS
$$
DECLARE
BEGIN
    PERFORM create_user_profile(NEW.id, NEW.username);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_create_user_profile ON users;
CREATE TRIGGER trigger_create_user_profile
    AFTER INSERT ON users
    FOR EACH ROW
    EXECUTE FUNCTION fn_trigger_create_user_profile();

DO $$ BEGIN
    PERFORM create_user_profile(id, username)
    FROM users;

    INSERT INTO user_profile_vods (
        user_id,
        video_uuid
    )
    SELECT u.id, v.video_uuid
    FROM vods AS v
    INNER JOIN users AS u
        ON u.uuid = v.user_uuid
    WHERE v.end_time IS NOT NULL AND v.is_local = FALSE;
END $$ LANGUAGE plpgsql;