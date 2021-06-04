DELETE FROM share_match_vod_connections AS mvc
WHERE NOT EXISTS (
    SELECT *
    FROM squad_role_assignments AS sra
    WHERE sra.squad_id = mvc.dest_squad_id
        AND sra.user_id = mvc.source_user_id
) AND mvc.dest_squad_id IS NOT NULL;

ALTER TABLE share_match_vod_connections
ADD CONSTRAINT share_match_vod_connections_dest_squad_id_source_user_id_fkey
FOREIGN KEY (dest_squad_id, source_user_id) REFERENCES squad_role_assignments(squad_id, user_id) ON DELETE CASCADE;