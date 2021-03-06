CREATE TABLE wow_match_transfer_log (
    match_uuid UUID UNIQUE REFERENCES matches(uuid) ON DELETE CASCADE
);