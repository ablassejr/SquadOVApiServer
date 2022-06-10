CREATE TABLE stripe_customers (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    customer VARCHAR NOT NULL
);