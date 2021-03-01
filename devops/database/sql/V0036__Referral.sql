CREATE EXTENSION IF NOT EXISTS citext;

CREATE TABLE referral_codes (
    id UUID PRIMARY KEY,
    code CITEXT NOT NULL UNIQUE,
    description VARCHAR NOT NULL,
    user_id BIGINT REFERENCES users(id) ON DELETE CASCADE
);

CREATE TABLE referral_downloads (
    code UUID NOT NULL,
    tm TIMESTAMPTZ NOT NULL
);

CREATE TABLE referral_visits (
    code UUID NOT NULL,
    tm TIMESTAMPTZ NOT NULL
);