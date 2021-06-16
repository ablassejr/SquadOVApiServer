DROP TABLE community_membership CASCADE;

CREATE TABLE community_membership (
    id BIGSERIAL PRIMARY KEY,
    community_id BIGINT NOT NULL REFERENCES communities(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sub_id BIGINT REFERENCES user_to_user_subscriptions(id) ON DELETE CASCADE,
    UNIQUE(community_id, user_id)
);

CREATE TABLE community_member_roles (
    membership_id BIGINT NOT NULL REFERENCES community_membership(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id BIGINT NOT NULL REFERENCES community_roles(id) ON DELETE CASCADE,
    UNIQUE(user_id, role_id)
);