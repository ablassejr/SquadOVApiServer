DELETE FROM user_event_record
WHERE user_id NOT IN (
    SELECT DISTINCT u.id
    FROM squadov.users AS u
    LEFT JOIN squadov.daily_active_sessions AS das
        ON das.user_id = u.id
            AND das.tm >= TO_TIMESTAMP(1625875200)
    WHERE das.user_id IS NULL
);