CREATE TABLE squads (
    id BIGSERIAL PRIMARY KEY,
    squad_name VARCHAR NOT NULL,
    squad_group VARCHAR NOT NULL UNIQUE,
    creation_time TIMESTAMPTZ NOT NULL,
    UNIQUE(squad_group, squad_name)
);

CREATE TABLE squad_membership_invites (
    squad_id BIGINT NOT NULL REFERENCES squads(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined BOOLEAN NOT NULL DEFAULT false,
    response_time TIMESTAMPTZ,
    invite_time TIMESTAMPTZ,
    UNIQUE(squad_id, user_id)
);

CREATE TYPE SQUAD_ROLE AS ENUM ('Owner', 'Member');
CREATE TABLE squad_role_assignments (
    squad_id BIGINT NOT NULL REFERENCES squads(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    squad_role SQUAD_ROLE NOT NULL,
    UNIQUE(squad_id, user_id)
);