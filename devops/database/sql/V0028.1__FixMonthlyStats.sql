DROP VIEW view_monthly_active_user_sessions;
DROP VIEW view_monthly_active_user_endpoint;

CREATE VIEW view_monthly_active_user_sessions (
    tm,
    users
)
AS
SELECT DATE_TRUNC('month', tm), COUNT(DISTINCT user_id)
FROM squadov.daily_active_sessions
GROUP BY DATE_TRUNC('month', tm);

CREATE VIEW view_monthly_active_user_endpoint (
    tm,
    users
)
AS
SELECT DATE_TRUNC('month', tm), COUNT(DISTINCT user_id)
FROM squadov.daily_active_endpoint
GROUP BY DATE_TRUNC('month', tm);