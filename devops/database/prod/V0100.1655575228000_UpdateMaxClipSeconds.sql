UPDATE squadov.user_feature_flags AS uf
SET max_clip_seconds = 180
FROM (
    SELECT user_id
    FROM squadov.user_subscription_tier
    WHERE tier = 'SILVER'
) AS sub
WHERE sub.user_id = uf.user_id;


UPDATE squadov.user_feature_flags AS uf
SET max_clip_seconds = 300
FROM (
    SELECT user_id
    FROM squadov.user_subscription_tier
    WHERE tier = 'GOLD' or tier = 'DIAMOND'
) AS sub
WHERE sub.user_id = uf.user_id;