CREATE TABLE wow_match_combat_log_association (
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    combat_log_uuid UUID NOT NULL REFERENCES wow_combat_logs (uuid) ON DELETE CASCADE,
    UNIQUE(match_uuid, combat_log_uuid)
);