CREATE TABLE user_autosharing_settings (
    id BIGSERIAL PRIMARY KEY,
    source_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dest_user_id BIGINT REFERENCES users(id) ON DELETE CASCADE,
    dest_squad_id BIGINT REFERENCES squads(id) ON DELETE CASCADE,
    can_share BOOLEAN NOT NULL,
    can_clip BOOLEAN NOT NULL,
    UNIQUE(source_user_id, dest_user_id),
    UNIQUE(source_user_id, dest_squad_id),
    FOREIGN KEY(dest_squad_id, source_user_id) REFERENCES squad_role_assignments(squad_id, user_id) ON DELETE CASCADE
);

CREATE INDEX ON user_autosharing_settings(source_user_id);

CREATE TABLE user_autosharing_settings_games (
    id BIGINT NOT NULL REFERENCES user_autosharing_settings(id) ON DELETE CASCADE,
    game INTEGER NOT NULL,
    UNIQUE(id, game)
);