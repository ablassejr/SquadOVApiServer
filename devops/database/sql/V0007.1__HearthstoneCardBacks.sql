CREATE TABLE hearthstone_card_backs (
    id INTEGER PRIMARY KEY,
    has_back BOOLEAN NOT NULL,
    active BOOLEAN NOT NULL
);

CREATE TABLE hearthstone_card_back_names (
    back_id INTEGER NOT NULL REFERENCES hearthstone_card_backs(id) ON DELETE CASCADE,
    locale VARCHAR NOT NULL,
    string VARCHAR NOT NULL,
    UNIQUE(back_id, locale)
);

CREATE TABLE hearthstone_card_back_descriptions (
    back_id INTEGER NOT NULL REFERENCES hearthstone_card_backs(id) ON DELETE CASCADE,
    locale VARCHAR NOT NULL,
    string VARCHAR NOT NULL,
    UNIQUE(back_id, locale)
);