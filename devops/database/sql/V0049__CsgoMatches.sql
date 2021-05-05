CREATE TABLE csgo_matches (
    match_uuid UUID UNIQUE NOT NULL REFERENCES matches (uuid) ON DELETE CASCADE,
    connected_server VARCHAR NOT NULL,
    tr TSTZRANGE NOT NULL,
    EXCLUDE USING GIST (match_uuid WITH <>, connected_server WITH =, tr WITH &&)
);