CREATE TABLE lol_matches (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    platform VARCHAR NOT NULL,
    match_id BIGINT NOT NULL,
    UNIQUE(match_id, platform)
);

CREATE TABLE tft_matches (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    platform VARCHAR NOT NULL,
    region VARCHAR NOT NULL,
    match_id BIGINT NOT NULL,
    UNIQUE(match_id, platform)
);