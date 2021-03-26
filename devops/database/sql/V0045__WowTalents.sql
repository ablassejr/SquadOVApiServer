CREATE TABLE wow_match_view_combatant_talents (
    event_id BIGINT NOT NULL REFERENCES wow_match_view_combatants(event_id) ON DELETE CASCADE,
    character_id BIGINT NOT NULL REFERENCES wow_match_view_character_presence(character_id) ON DELETE CASCADE,
    talent_id INTEGER NOT NULL,
    is_pvp BOOLEAN NOT NULL,
    FOREIGN KEY (event_id, character_id) REFERENCES wow_match_view_combatants(event_id, character_id)
);

CREATE INDEX ON wow_match_view_combatant_talents(event_id);