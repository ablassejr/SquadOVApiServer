ALTER TABLE user_feature_flags
ADD COLUMN disable_es_search BOOLEAN DEFAULT FALSE;