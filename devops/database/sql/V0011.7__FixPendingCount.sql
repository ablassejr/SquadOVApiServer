
CREATE OR REPLACE VIEW squad_pending_invite_count AS
SELECT sq.id, COUNT(smi.invite_uuid) AS "pending_invite_count" 
FROM squads AS sq
LEFT JOIN squad_membership_invites AS smi
    ON sq.id = smi.squad_id AND smi.response_time IS NULL
GROUP BY sq.id;