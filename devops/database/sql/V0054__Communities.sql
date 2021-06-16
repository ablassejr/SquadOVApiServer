CREATE TABLE communities (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR UNIQUE NOT NULL,
    create_tm TIMESTAMPTZ NOT NULL,
    creator_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- 0 (public), 1 (protected), 2 (private)
    security_level INTEGER NOT NULL,
    requires_subscription BOOLEAN NOT NULL DEFAULT FALSE,
    allow_twitch_sub BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE community_roles (
    id BIGSERIAL PRIMARY KEY,
    community_id BIGINT NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    can_manage BOOLEAN NOT NULL DEFAULT FALSE,
    can_moderate BOOLEAN NOT NULL DEFAULT FALSE,
    can_invite BOOLEAN NOT NULL DEFAULT FALSE,
    can_share BOOLEAN NOT NULL DEFAULT FALSE,
    UNIQUE(community_id, name)
);

CREATE TABLE user_to_user_subscriptions (
    id BIGSERIAL PRIMARY KEY,
    source_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    dest_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_twitch BOOLEAN NOT NULL DEFAULT FALSE,
    last_checked TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON user_to_user_subscriptions(source_user_id, dest_user_id);

CREATE TABLE community_membership (
    id BIGSERIAL PRIMARY KEY,
    role_id BIGINT NOT NULL REFERENCES community_roles(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sub_id BIGINT REFERENCES user_to_user_subscriptions(id) ON DELETE CASCADE
);

CREATE INDEX ON community_membership(user_id, role_id);