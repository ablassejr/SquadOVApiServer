CREATE TABLE community_invites (
    code UUID PRIMARY KEY,
    community_id BIGINT NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
    inviter_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    num_uses INTEGER NOT NULL DEFAULT 0,
    max_uses INTEGER,
    expiration TIMESTAMPTZ,
    created_tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON community_invites(community_id, inviter_user_id);

CREATE TABLE community_invite_usage (
    code UUID NOT NULL REFERENCES community_invites(code) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    usage_tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON community_invite_usage(code);