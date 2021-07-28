CREATE TABLE squad_invite_links (
    id BIGSERIAL PRIMARY KEY,
    squad_id BIGINT NOT NULL REFERENCES squads(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    create_time TIMESTAMPTZ NOT NULL,
    expire_time TIMESTAMPTZ,
    max_uses INTEGER
);

CREATE INDEX ON squad_invite_links(squad_id, user_id);

CREATE TABLE squad_invite_link_usage (
    link_id BIGINT NOT NULL REFERENCES squad_invite_links(id),
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    usage_time TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON squad_invite_link_usage(link_id, user_id);