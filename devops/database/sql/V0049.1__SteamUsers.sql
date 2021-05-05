CREATE TABLE steam_users_cache (
    steam_id BIGINT PRIMARY KEY,
    steam_name VARCHAR NOT NULL
);

CREATE TABLE steam_user_links (
    steam_id BIGINT NOT NULL REFERENCES steam_users_cache(steam_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE
);