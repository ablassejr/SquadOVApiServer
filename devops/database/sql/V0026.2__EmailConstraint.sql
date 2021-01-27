ALTER TABLE squad_membership_invites
DROP CONSTRAINT squad_membership_invites_squad_id_email_key;

DROP INDEX squad_invite_one_pending_invite_idx;

CREATE UNIQUE INDEX squad_invite_one_pending_invite_user_id_idx ON squad_membership_invites (squad_id, user_id)
WHERE response_time IS NULL AND user_id IS NOT NULL;

CREATE UNIQUE INDEX squad_invite_one_pending_invite_email_idx ON squad_membership_invites (squad_id, email)
WHERE response_time IS NULL;