DROP TRIGGER IF EXISTS trigger_update_vod_fill_events ON vods;
CREATE TRIGGER trigger_update_vod_fill_events
    AFTER INSERT OR UPDATE ON vods
    FOR EACH ROW
    EXECUTE FUNCTION fn_trigger_update_vod_fill_events();