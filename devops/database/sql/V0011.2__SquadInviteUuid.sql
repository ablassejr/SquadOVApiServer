ALTER TABLE squad_membership_invites
ADD COLUMN invite_uuid UUID NOT NULL DEFAULT gen_random_uuid();

ALTER TABLE squad_membership_invites
ADD CONSTRAINT squad_membership_invites_squad_id_invite_uuid_key UNIQUE(squad_id, invite_uuid);