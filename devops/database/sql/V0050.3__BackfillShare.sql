INSERT INTO share_match_vod_connections (
    match_uuid,
    video_uuid,
    source_user_id,
    dest_user_id,
    dest_squad_id,
    can_share,
    can_clip,
    parent_connection_id,
    share_depth
)
SELECT DISTINCT ON (v.match_uuid, v.video_uuid, u.id, sra.squad_id)
    v.match_uuid,
    v.video_uuid,
    u.id,
    NULL,
    sra.squad_id,
    FALSE,
    FALSE,
    NULL,
    0
FROM vods AS v
INNER JOIN users AS u
    ON v.user_uuid = u.uuid
INNER JOIN squad_role_assignments AS sra
    ON sra.user_id = u.id
WHERE v.match_uuid IS NOT NULL
    AND v.is_clip = FALSE;

INSERT INTO share_match_vod_connections (
    match_uuid,
    video_uuid,
    source_user_id,
    dest_user_id,
    dest_squad_id,
    can_share,
    can_clip,
    parent_connection_id,
    share_depth
)
SELECT DISTINCT ON (v.video_uuid, u.id, sra.squad_id)
    NULL,
    v.video_uuid,
    u.id,
    NULL,
    sra.squad_id,
    FALSE,
    FALSE,
    NULL,
    0
FROM vods AS v
INNER JOIN users AS u
    ON v.user_uuid = u.uuid
INNER JOIN squad_role_assignments AS sra
    ON sra.user_id = u.id
WHERE v.match_uuid IS NOT NULL
    AND v.is_clip = TRUE;