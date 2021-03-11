CREATE INDEX CONCURRENTLY ON wow_match_view_events (view_id, log_line);
DROP INDEX CONCURRENTLY wow_match_view_events_view_id_idx;