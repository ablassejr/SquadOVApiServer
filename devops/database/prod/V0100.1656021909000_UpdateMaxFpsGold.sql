UPDATE squadov.user_feature_flags AS uf
SET max_record_fps = 90
FROM (
    SELECT user_id
    FROM squadov.user_subscription_tier
    WHERE tier = 'GOLD'
) AS sub
WHERE sub.user_id = uf.user_id;