INSERT INTO squadov.user_event_record (
    user_id,
    event_name,
    tm
)
SELECT u.id, 'inactive_14', NOW()
FROM squadov.users AS u
LEFT JOIN squadov.daily_active_sessions AS das
    ON das.user_id = u.id
        AND das.tm >= TO_TIMESTAMP(1625875200)
WHERE das.user_id IS NULL
GROUP BY u.id;