CREATE TABLE squadov.new_share_match_vod_connections AS 
SELECT mvc.*
FROM squadov.share_match_vod_connections AS mvc
LEFT JOIN squadov.squad_role_assignments AS sra
    ON sra.squad_id = mvc.dest_squad_id
        AND sra.user_id = mvc.source_user_id
WHERE sra.squad_id IS NOT NULL
    AND mvc.dest_squad_id IS NOT NULL
UNION
SELECT mvc.*
FROM squadov.share_match_vod_connections AS mvc
WHERE mvc.dest_squad_id IS NULL;

ALTER TABLE squadov.new_share_match_vod_connections
ADD CONSTRAINT new_share_match_vod_connections_dest_squad_id_source_user_id_fk
FOREIGN KEY (dest_squad_id, source_user_id) REFERENCES squad_role_assignments(squad_id, user_id) ON DELETE CASCADE;

CREATE OR REPLACE VIEW squadov.view_share_connections_access_users (
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
    FROM squadov.new_share_match_vod_connections AS mvc
    WHERE mvc.dest_user_id IS NOT NULL
    UNION
    SELECT mvc.id, mvc.match_uuid, mvc.video_uuid, sra.user_id, mvc.share_depth, mvc.parent_connection_id, mvc.source_user_id
    FROM squadov.new_share_match_vod_connections AS mvc
    INNER JOIN squadov.squad_role_assignments AS sra
        ON sra.squad_id = mvc.dest_squad_id
    WHERE mvc.dest_squad_id IS NOT NULL
        AND sra.user_id != mvc.source_user_id
) AS f(id, match_uuid, video_uuid, user_id, share_depth, parent_connection_id, source_user_id)
ORDER BY f.match_uuid, f.video_uuid, f.user_id, f.share_depth ASC;

DROP TABLE squadov.share_match_vod_connections CASCADE;
ALTER TABLE squadov.new_share_match_vod_connections RENAME TO share_match_vod_connections;