DELETE FROM matches
WHERE uuid IN (
    SELECT match_uuid
    FROM wow_arenas
    UNION
    SELECT match_uuid
    FROM wow_encounters
    UNION
    SELECT match_uuid
    FROM wow_challenges
);
DROP TABLE wow_arenas CASCADE;
DROP TABLE wow_encounters CASCADE;
DROP TABLE wow_challenges CASCADE;
DROP TABLE wow_combat_log_events CASCADE;
DROP TABLE wow_combat_logs CASCADE;
DROP TABLE wow_combat_log_character_presence CASCADE;
DROP TABLE wow_combatlog_unit_ownership CASCADE;
DROP TABLE wow_match_combatants CASCADE;
DROP TABLE wow_match_combat_log_association CASCADE;
DROP TABLE wow_user_character_association CASCADE;