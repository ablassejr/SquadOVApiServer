CREATE TABLE wow_match_view_combatant_covenants (
    event_id BIGINT NOT NULL REFERENCES wow_match_view_combatants(event_id) ON DELETE CASCADE,
    character_id BIGINT NOT NULL REFERENCES wow_match_view_character_presence(character_id) ON DELETE CASCADE,
    covenant_id INTEGER NOT NULL,
    soulbind_id INTEGER NOT NULL,
    soulbind_traits INTEGER[] NOT NULL,
    conduit_item_ids INTEGER[] NOT NULL,
    conduit_item_ilvls INTEGER[] NOT NULL,
    FOREIGN KEY (event_id, character_id) REFERENCES wow_match_view_combatants(event_id, character_id)
);

CREATE INDEX ON wow_match_view_combatant_covenants(event_id);