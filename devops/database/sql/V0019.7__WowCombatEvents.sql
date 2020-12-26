CREATE EXTENSION btree_gin;
CREATE TABLE wow_combat_log_events (
    combat_log_uuid UUID NOT NULL REFERENCES wow_combat_logs (uuid) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL,
    -- Source/dest MAY be null because of the COMBATANT_INFO event
    source JSONB,
    dest JSONB,
    -- NULL if advanced logging isn't turned on as those values would be garbage anyway.
    advanced JSONB,
    evt JSONB NOT NULL
);

CREATE INDEX ON wow_combat_log_events USING gin(combat_log_uuid, tm, evt jsonb_path_ops, source jsonb_path_ops);
CREATE INDEX ON wow_combat_log_events USING gin(combat_log_uuid, tm, evt jsonb_path_ops, dest jsonb_path_ops);