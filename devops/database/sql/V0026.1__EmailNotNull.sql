UPDATE squad_membership_invites
SET email=smi.email
FROM (
    SELECT smi.invite_uuid, u.email
    FROM squad_membership_invites AS smi
    INNER JOIN users AS u
        ON u.id = smi.user_id
) AS smi
WHERE squad_membership_invites.invite_uuid = smi.invite_uuid;

ALTER TABLE squad_membership_invites
ALTER COLUMN email SET NOT NULL;