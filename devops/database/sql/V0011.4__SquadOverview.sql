CREATE OR REPLACE VIEW squad_overview AS
SELECT sq.*, COUNT(sra.user_id) AS "member_count"
FROM squads AS sq
INNER JOIN squad_role_assignments AS sra
    ON sq.id = sra.squad_id
GROUP BY sq.id