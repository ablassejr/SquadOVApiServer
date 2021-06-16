ALTER TABLE community_member_roles
DROP COLUMN user_id CASCADE;

ALTER TABLE community_member_roles
ADD CONSTRAINT community_member_roles_membership_id_role_id_idx
UNIQUE (membership_id, role_id);