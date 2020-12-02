CREATE OR REPLACE VIEW squad_member_count AS
SELECT sq.id, COUNT(sra.user_id) AS "member_count" 
FROM squads AS sq
INNER JOIN squad_role_assignments AS sra
    ON sq.id = sra.squad_id
GROUP BY sq.id;

CREATE OR REPLACE VIEW squad_pending_invite_count AS
SELECT sq.id, COUNT(smi.invite_uuid) AS "pending_invite_count" 
FROM squads AS sq
INNER JOIN squad_membership_invites AS smi
    ON sq.id = smi.squad_id AND smi.response_time IS NULL
GROUP BY sq.id;

CREATE OR REPLACE VIEW squad_overview AS
SELECT
    sq.*,
    smc.member_count,
    spic.pending_invite_count
FROM squads AS sq
INNER JOIN squad_member_count AS smc
    ON smc.id = sq.id
INNER JOIN squad_pending_invite_count AS spic
    ON spic.id = sq.id;