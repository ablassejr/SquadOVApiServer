ALTER TABLE squad_membership_invites
DROP CONSTRAINT squad_membership_invites_squad_id_user_id_key;

CREATE UNIQUE INDEX squad_invite_one_pending_invite_idx ON squad_membership_invites (squad_id, user_id)
WHERE response_time IS NULL;
