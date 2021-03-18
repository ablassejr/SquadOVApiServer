ALTER TABLE squads
ADD COLUMN is_default BOOLEAN NOT NULL DEFAULT FALSE;

UPDATE squads
SET is_default = TRUE
WHERE id IN (
    SELECT s.id
    FROM squads AS s
    INNER JOIN squad_role_assignments AS sra
        ON sra.squad_id = s.id
    INNER JOIN users AS u
        ON u.id = sra.user_id
    WHERE s.squad_group = u.username
        AND sra.squad_role = 'Owner'
        AND s.squad_name = u.username || '''s Squad'
);