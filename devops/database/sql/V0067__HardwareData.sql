CREATE TABLE user_hardware_specs (
    user_id BIGINT UNIQUE NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    os JSONB NOT NULL,
    cpu JSONB NOT NULL,
    display JSONB NOT NULL,
    ram_kb BIGINT NOT NULL
);