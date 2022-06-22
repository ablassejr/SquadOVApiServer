UPDATE squadov.match_custom_events AS mce
SET video_uuid = sub.video_uuid
FROM (
    SELECT mce.event_id, v.video_uuid
    FROM squadov.match_custom_events AS mce
    INNER JOIN squadov.users AS u
        ON u.id = mce.user_id
    INNER JOIN squadov.vods AS v
        ON v.match_uuid = mce.match_uuid
            AND v.user_uuid = u.uuid
            AND NOT v.is_clip
    WHERE mce.video_uuid IS NULL
) AS sub
WHERE mce.event_id = sub.event_id;

DELETE FROM squadov.match_custom_events
WHERE video_uuid IS NULL;

ALTER TABLE squadov.match_custom_events
ALTER COLUMN video_uuid SET NOT NULL;