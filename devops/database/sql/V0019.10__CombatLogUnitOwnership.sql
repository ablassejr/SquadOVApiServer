CREATE TABLE wow_user_character_association (
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    guid VARCHAR NOT NULL,
    UNIQUE(user_id, guid)
);

CREATE TABLE wow_combatlog_unit_ownership (
    combat_log_uuid UUID NOT NULL REFERENCES wow_combat_logs (uuid) ON DELETE CASCADE,
    unit_guid VARCHAR NOT NULL,
    owner_guid VARCHAR NOT NULL,
    UNIQUE(combat_log_uuid, owner_guid, unit_guid)
);