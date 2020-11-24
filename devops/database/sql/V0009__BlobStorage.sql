CREATE TABLE blob_link_storage (
    uuid UUID PRIMARY KEY,
    bucket VARCHAR NOT NULL,
    local_path VARCHAR NOT NULL
);