ALTER TABLE squad_membership_invites
ADD COLUMN inviter_user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE;