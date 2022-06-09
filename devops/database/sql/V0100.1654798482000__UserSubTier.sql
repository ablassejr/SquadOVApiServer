CREATE TABLE user_subscription_tier (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    tier VARCHAR NOT NULL
);