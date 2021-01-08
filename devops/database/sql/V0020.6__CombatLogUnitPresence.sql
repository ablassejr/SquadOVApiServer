CREATE TABLE wow_combat_log_character_presence (
    combat_log_uuid UUID NOT NULL REFERENCES wow_combat_logs (uuid) ON DELETE CASCADE,
    guid VARCHAR NOT NULL,
    UNIQUE(combat_log_uuid, guid)
);