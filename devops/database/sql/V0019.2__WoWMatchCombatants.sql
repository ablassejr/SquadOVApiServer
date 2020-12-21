CREATE TABLE wow_match_combatants (
    match_uuid UUID NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    combatant_guid VARCHAR NOT NULL,
    UNIQUE(match_uuid, combatant_guid)
);