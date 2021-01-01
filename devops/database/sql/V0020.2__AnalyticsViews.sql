CREATE VIEW view_daily_active_vod_users (
    tm,
    users
)
AS
SELECT DATE_TRUNC('day', start_time), COUNT(DISTINCT user_uuid)
FROM squadov.vods
WHERE start_time IS NOT NULL AND user_uuid IS NOT NULL
GROUP BY DATE_TRUNC('day', start_time)
ORDER BY DATE_TRUNC('day', start_time);

CREATE VIEW view_daily_active_user_sessions (
    tm,
    users
)
AS
SELECT tm, COUNT(user_id)
FROM squadov.daily_active_sessions
GROUP BY tm;

CREATE VIEW view_daily_active_user_endpoint (
    tm,
    users
)
AS
SELECT tm, COUNT(user_id)
FROM squadov.daily_active_endpoint
GROUP BY tm;

CREATE VIEW view_monthly_active_vod_users (
    tm,
    users
)
AS
SELECT DATE_TRUNC('month', start_time), COUNT(DISTINCT user_uuid)
FROM squadov.vods
WHERE start_time IS NOT NULL AND user_uuid IS NOT NULL
GROUP BY DATE_TRUNC('month', start_time)
ORDER BY DATE_TRUNC('month', start_time);

CREATE VIEW view_monthly_active_user_sessions (
    tm,
    users
)
AS
SELECT DATE_TRUNC('month', tm), COUNT(user_id)
FROM squadov.daily_active_sessions
GROUP BY DATE_TRUNC('month', tm);

CREATE VIEW view_monthly_active_user_endpoint (
    tm,
    users
)
AS
SELECT DATE_TRUNC('month', tm), COUNT(user_id)
FROM squadov.daily_active_endpoint
GROUP BY DATE_TRUNC('month', tm);