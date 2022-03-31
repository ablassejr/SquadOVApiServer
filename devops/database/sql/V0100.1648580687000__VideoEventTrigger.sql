CREATE OR REPLACE FUNCTION fn_trigger_update_vod_fill_events()
    RETURNS trigger AS
$$
DECLARE
BEGIN
    IF NEW.match_uuid IS NOT NULL THEN
        UPDATE squadov.match_custom_events
        SET match_uuid = NEW.match_uuid
        WHERE video_uuid = NEW.video_uuid;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_vod_fill_events ON vods;
CREATE TRIGGER trigger_update_vod_fill_events
    AFTER INSERT OR UPDATE ON vods
    FOR EACH ROW
    EXECUTE FUNCTION fn_trigger_new_squad_auto_share();