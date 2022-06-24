DROP TABLE squadov.wow_match_view_aura_break_events CASCADE;
DROP TABLE squadov.wow_match_view_aura_events CASCADE;
DROP TABLE squadov.wow_match_view_character_presence CASCADE;
DROP TABLE squadov.wow_match_view_combatant_covenants CASCADE;
DROP TABLE squadov.wow_match_view_combatant_items CASCADE;
DROP TABLE squadov.wow_match_view_combatant_talents CASCADE;
DROP TABLE squadov.wow_match_view_combatants CASCADE;
DROP TABLE squadov.wow_match_view_damage_events CASCADE;
DROP TABLE squadov.wow_match_view_death_events CASCADE;
DROP TABLE squadov.wow_match_view_healing_events CASCADE;
DROP TABLE squadov.wow_match_view_resurrect_events CASCADE;
DROP TABLE squadov.wow_match_view_spell_cast_events CASCADE;
DROP TABLE squadov.wow_match_view_subencounter_events CASCADE;
DROP TABLE squadov.wow_match_view_summon_events CASCADE;
DROP TABLE squadov.wow_match_view_events CASCADE;
DROP TABLE squadov.wow_match_transfer_log CASCADE;

ALTER TABLE squadov.wow_match_view
DROP COLUMN player_rating,
DROP COLUMN player_spec,
DROP COLUMN player_team,
DROP COLUMN t0_specs,
DROP COLUMN t1_specs;