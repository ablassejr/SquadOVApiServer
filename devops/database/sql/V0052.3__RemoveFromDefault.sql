DELETE FROM squad_role_assignments AS sra
USING squads AS s
WHERE s.id = sra.squad_id
    AND s.is_default
    AND (sra.user_id = 1 OR sra.user_id = 4);