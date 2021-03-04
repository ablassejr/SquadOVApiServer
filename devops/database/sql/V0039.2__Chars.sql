CREATE TABLE wow_match_view_character_presence (
    character_id BIGSERIAL PRIMARY KEY,
    view_id UUID NOT NULL REFERENCES wow_match_view(id) ON DELETE CASCADE,
    unit_guid VARCHAR NOT NULL,
    owner_guid VARCHAR,
    flags BIGINT NOT NULL,
    has_combatant_info BOOLEAN NOT NULL,
    UNIQUE(view_id, character_id),
    UNIQUE(view_id, unit_guid)
);

CREATE INDEX ON wow_match_view_character_presence(view_id, owner_guid);

CREATE TABLE wow_match_view_events (
    event_id BIGSERIAL PRIMARY KEY,
    view_id UUID NOT NULL REFERENCES wow_match_view(id) ON DELETE CASCADE,
    source_char BIGINT REFERENCES wow_match_view_character_presence(character_id) ON DELETE CASCADE,
    dest_char BIGINT REFERENCES wow_match_view_character_presence(character_id) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON wow_match_view_events(view_id);

CREATE TABLE wow_match_view_combatants (
    event_id BIGINT UNIQUE NOT NULL REFERENCES wow_match_view_events(event_id) ON DELETE CASCADE,
    character_id BIGINT NOT NULL REFERENCES wow_match_view_character_presence(character_id) ON DELETE CASCADE,
    team INTEGER NOT NULL,
    spec_id INTEGER NOT NULL,
    UNIQUE(event_id, character_id)
);

CREATE TABLE wow_match_view_combatant_items (
    event_id BIGINT NOT NULL REFERENCES wow_match_view_combatants(event_id) ON DELETE CASCADE,
    character_id BIGINT NOT NULL REFERENCES wow_match_view_character_presence(character_id) ON DELETE CASCADE,
    idx INTEGER NOT NULL,
    item_id BIGINT NOT NULL,
    ilvl INTEGER NOT NULL,
    FOREIGN KEY (event_id, character_id) REFERENCES wow_match_view_combatants(event_id, character_id),
    UNIQUE(event_id, idx)
);

CREATE TABLE wow_user_character_cache (
    user_id BIGINT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    event_id BIGINT NOT NULL REFERENCES wow_match_view_combatants(event_id) ON DELETE RESTRICT
);