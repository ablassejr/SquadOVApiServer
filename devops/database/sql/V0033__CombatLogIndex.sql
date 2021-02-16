CREATE INDEX CONCURRENTLY ON wow_combat_log_events (combat_log_uuid, tm, (evt->>'type'), (evt->>'guid'));
DROP INDEX CONCURRENTLY wow_combat_log_events_combat_log_uuid_tm_evt_dest_idx;
DROP INDEX CONCURRENTLY wow_combat_log_events_combat_log_uuid_tm_evt_source_idx;