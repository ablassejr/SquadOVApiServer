CREATE TABLE user_referral_code_usage (
    user_id BIGINT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    code_id UUID NOT NULL REFERENCES referral_codes(id) ON DELETE CASCADE,
    tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON user_referral_code_usage(code_id);