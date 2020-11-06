ALTER TABLE hearthstone_matches
DROP COLUMN deck_name;

ALTER TABLE hearthstone_player_match_decks
ADD COLUMN deck_name VARCHAR NOT NULL;