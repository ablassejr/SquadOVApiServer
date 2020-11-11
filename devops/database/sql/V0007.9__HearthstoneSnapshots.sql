CREATE TABLE hearthstone_snapshots (
    snapshot_id UUID PRIMARY KEY,
    match_uuid UUID UNIQUE NOT NULL REFERENCES hearthstone_matches (match_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    last_action_id BIGINT NOT NULL,
    tm TIMESTAMPTZ,
    game_entity_id INTEGER NOT NULL,
    current_turn INTEGER NOT NULL,
    step INTEGER NOT NULL,
    current_player_id INTEGER NOT NULL,
    FOREIGN KEY(match_uuid, user_id, last_action_id) REFERENCES hearthstone_actions(match_uuid, user_id, action_id) ON DELETE CASCADE,
    UNIQUE(match_uuid, user_id, last_action_id)
);

CREATE TABLE hearthstone_snapshots_player_map(
    snapshot_id UUID NOT NULL REFERENCES hearthstone_snapshots(snapshot_id) ON DELETE CASCADE,
    player_name VARCHAR NOT NULL,
    player_id INTEGER NOT NULL,
    entity_id INTEGER NOT NULL,
    UNIQUE(snapshot_id, player_id)
);

CREATE TABLE hearthstone_snapshots_entities(
    snapshot_id UUID NOT NULL REFERENCES hearthstone_snapshots(snapshot_id) ON DELETE CASCADE,
    entity_id INTEGER NOT NULL,
    tags JSONB NOT NULL,
    attributes JSONB NOT NULL,
    UNIQUE(snapshot_id, entity_id)
);