ALTER TABLE user_sessions
ADD COLUMN transition_id VARCHAR(36),
ADD COLUMN issue_tm TIMESTAMPTZ,
ADD COLUMN expiration_tm TIMESTAMPTZ;

INSERT INTO user_sessions (
    id,
    access_token,
    refresh_token,
    user_id,
    is_temp,
    transition_id,
    issue_tm,
    expiration_tm
)
SELECT
    old_id,
    access_token,
    refresh_token,
    user_id,
    is_temp,
    id,
    NOW(),
    NOW() + INTERVAL '3 hour'
FROM user_sessions
WHERE old_id IS NOT NULL;

ALTER TABLE user_sessions
DROP COLUMN old_id CASCADE;