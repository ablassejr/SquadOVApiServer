CREATE TABLE wow_spell_to_class (
    build_id VARCHAR NOT NULL,
    spell_id INTEGER NOT NULL,
    class_id INTEGER NOT NULL,
    UNIQUE(build_id, spell_id)
);