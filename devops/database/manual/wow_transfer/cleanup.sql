DELETE FROM squadov.matches
WHERE uuid IN (
    SELECT match_uuid
    FROM squadov.wow_arenas
    UNION
    SELECT match_uuid
    FROM squadov.wow_encounters
    UNION
    SELECT match_uuid
    FROM squadov.wow_challenges
);
DROP TABLE squadov.wow_arenas CASCADE;
DROP TABLE squadov.wow_encounters CASCADE;
DROP TABLE squadov.wow_challenges CASCADE;
DROP TABLE squadov.wow_combat_log_events CASCADE;
DROP TABLE squadov.wow_combat_logs CASCADE;
DROP TABLE squadov.wow_combat_log_character_presence CASCADE;
DROP TABLE squadov.wow_combatlog_unit_ownership CASCADE;
DROP TABLE squadov.wow_match_combatants CASCADE;
DROP TABLE squadov.wow_match_combat_log_association CASCADE;
DROP TABLE squadov.wow_user_character_association CASCADE;