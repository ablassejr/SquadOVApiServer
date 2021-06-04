CREATE OR REPLACE FUNCTION fn_trigger_new_squad_auto_share()
    RETURNS trigger AS
$$
DECLARE
BEGIN
    INSERT INTO user_autosharing_settings (
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

    INSERT INTO user_autosharing_settings_games (
        id,
        game
    )
    SELECT uas.id, g.game
    FROM user_autosharing_settings AS uas
    CROSS JOIN UNNEST(ARRAY[0,1,2,3,4,5,6]) AS g(game)
    WHERE uas.source_user_id = NEW.user_id AND uas.dest_squad_id = NEW.squad_id
    ON CONFLICT DO NOTHING;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_new_squad_auto_share ON squad_role_assignments;
CREATE TRIGGER trigger_new_squad_auto_share
    AFTER INSERT ON squad_role_assignments
    FOR EACH ROW
    EXECUTE FUNCTION fn_trigger_new_squad_auto_share();