CREATE TABLE geo_ip_cache (
    ip_addr CIDR NOT NULL,
    city VARCHAR,
    country VARCHAR,
    timezone INTEGER,
    cache_tm TIMESTAMPTZ NOT NULL
);

CREATE INDEX ON geo_ip_cache USING GIST(ip_addr);