UPDATE squadov.vods v
SET start_time = sub.start_time + sub.start_offset_ms * INTERVAL '1 millisecond',
    end_time = sub.start_time + sub.end_offset_ms * INTERVAL '1 millisecond'
FROM (
    SELECT sc.clip_uuid, v.start_time, sc.start_offset_ms, sc.end_offset_ms
    FROM squadov.staged_clips AS sc
    INNER JOIN squadov.vods AS v
        ON v.video_uuid = sc.video_uuid
    WHERE sc.clip_uuid IS NOT NULL
) AS sub
WHERE v.video_uuid = sub.clip_uuid
    AND v.start_time IS NULL
    AND v.end_time IS NULL