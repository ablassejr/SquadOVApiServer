CREATE TABLE wow_connected_realm_members (
    connected_realm_id BIGINT NOT NULL REFERENCES wow_connected_realms(id),
    realm_id BIGINT NOT NULL REFERENCES wow_realms(id),
    UNIQUE(connected_realm_id, realm_id)
);

CREATE EXTENSION IF NOT EXISTS fuzzystrmatch;