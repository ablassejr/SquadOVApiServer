CREATE TABLE wow_connected_realms (
    id BIGINT PRIMARY KEY,
    region VARCHAR NOT NULL
);
CREATE INDEX ON wow_connected_realms(region);