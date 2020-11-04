CREATE TABLE hearthstone_cards (
    card_id VARCHAR PRIMARY KEY,
    id INTEGER NOT NULL,
    has_card BOOLEAN NOT NULL,
    has_golden BOOLEAN NOT NULL
);

CREATE TABLE hearthstone_card_names (
    card_id VARCHAR NOT NULL REFERENCES hearthstone_cards(card_id) ON DELETE CASCADE,
    locale VARCHAR NOT NULL,
    string VARCHAR NOT NULL,
    UNIQUE(card_id, locale)
);

CREATE TABLE hearthstone_card_text (
    card_id VARCHAR NOT NULL REFERENCES hearthstone_cards(card_id) ON DELETE CASCADE,
    locale VARCHAR NOT NULL,
    string VARCHAR NOT NULL,
    UNIQUE(card_id, locale)
);

CREATE TABLE hearthstone_card_tags (
    card_id VARCHAR NOT NULL REFERENCES hearthstone_cards(card_id) ON DELETE CASCADE,
    tag INTEGER NOT NULL,
    val INTEGER NOT NULL,
    UNIQUE(card_id, tag)
);