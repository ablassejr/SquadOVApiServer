CREATE OR REPLACE VIEW view_share_connections_access_users (
    id,
    match_uuid,
    video_uuid,
    user_id,
    parent_connection_id,
    source_user_id
)
AS
SELECT
    f.id,
    f.match_uuid,
    f.video_uuid,
    f.user_id,
    f.parent_connection_id,
    f.source_user_id
FROM (
    SELECT mvc.id, mvc.match_uuid, mvc.video_uuid, mvc.dest_user_id, mvc.share_depth, mvc.parent_connection_id, mvc.source_user_id
    FROM share_match_vod_connections AS mvc
    WHERE mvc.dest_user_id IS NOT NULL
    UNION
    SELECT mvc.id, mvc.match_uuid, mvc.video_uuid, sra.user_id, mvc.share_depth, mvc.parent_connection_id, mvc.source_user_id
    FROM share_match_vod_connections AS mvc
    INNER JOIN squadov.squad_role_assignments AS sra
        ON sra.squad_id = mvc.dest_squad_id
    WHERE mvc.dest_squad_id IS NOT NULL
) AS f(id, match_uuid, video_uuid, user_id, share_depth, parent_connection_id, source_user_id)
ORDER BY f.match_uuid, f.video_uuid, f.user_id, f.share_depth ASC;