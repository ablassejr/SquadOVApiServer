CREATE TABLE twitch_event_subscriptions (
    id VARCHAR PRIMARY KEY,
    sub VARCHAR NOT NULL,
    raw_data JSONB NOT NULL
);
CREATE INDEX ON twitch_event_subscriptions(sub);