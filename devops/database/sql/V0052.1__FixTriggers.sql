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
        AND asg.id IS NOT NULL;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION fn_trigger_new_squad_auto_share()
    RETURNS trigger AS
$$
DECLARE
BEGIN
    INSERT INTO squadov.user_autosharing_settings (
        source_user_id,
        dest_user_id,
        dest_squad_id,
        can_share,
        can_clip
    )
    VALUES (
        NEW.user_id,
        NULL,
        NEW.squad_id,
        TRUE,
        TRUE
    )
    ON CONFLICT DO NOTHING;

    INSERT INTO squadov.user_autosharing_settings_games (
        id,
        game
    )
    SELECT uas.id, g.game
    FROM squadov.user_autosharing_settings AS uas
    CROSS JOIN UNNEST(ARRAY[0,1,2,3,4,5,6]) AS g(game)
    WHERE uas.source_user_id = NEW.user_id AND uas.dest_squad_id = NEW.squad_id
    ON CONFLICT DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;