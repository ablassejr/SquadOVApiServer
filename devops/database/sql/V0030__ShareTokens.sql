CREATE TABLE share_tokens (
    id UUID PRIMARY KEY,
    match_uuid UUID NOT NULL REFERENCES lol_matches (match_uuid) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    encrypted_token VARCHAR NOT NULL,
    iv VARCHAR NOT NULL,
    UNIQUE(match_uuid, user_id)
);