ALTER TABLE squad_membership_invites
ALTER COLUMN user_id DROP NOT NULL,
ADD COLUMN email VARCHAR,
ADD CONSTRAINT squad_membership_invites_squad_id_email_key UNIQUE(squad_id, email);