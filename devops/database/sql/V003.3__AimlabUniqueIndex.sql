ALTER TABLE aimlab_tasks
ADD COLUMN klutch_id VARCHAR NOT NULL;

-- Going to just assume that the 'id' column will be unique given a Klutch ID...pls.
ALTER TABLE aimlab_tasks
ADD CONSTRAINT aimlab_player_unique UNIQUE (user_id, klutch_id, id);