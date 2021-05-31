CREATE OR REPLACE VIEW view_share_connections_access_users (
    id,
    match_uuid,
    video_uuid,
    user_id
)
AS
SELECT DISTINCT ON (match_uuid, video_uuid, user_id)
    f.id,
    f.match_uuid AS match_uuid,
    f.video_uuid AS video_uuid,
    f.user_id AS user_id
FROM (
    SELECT mvc.id, mvc.match_uuid, mvc.video_uuid, mvc.dest_user_id, mvc.share_depth
    FROM share_match_vod_connections AS mvc
    WHERE mvc.dest_user_id IS NOT NULL
    UNION
    SELECT mvc.id, mvc.match_uuid, mvc.video_uuid, sra.user_id, mvc.share_depth
    FROM share_match_vod_connections AS mvc
    INNER JOIN squadov.squad_role_assignments AS sra
        ON sra.squad_id = mvc.dest_squad_id
    WHERE mvc.dest_squad_id IS NOT NULL
) AS f(id, match_uuid, video_uuid, user_id, share_depth)
ORDER BY f.match_uuid, f.video_uuid, f.user_id, f.share_depth ASC;