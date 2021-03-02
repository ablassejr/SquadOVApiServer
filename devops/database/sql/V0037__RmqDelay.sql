CREATE TABLE deferred_rabbitmq_messages (
    id BIGSERIAL PRIMARY KEY,
    execute_time TIMESTAMPTZ NOT NULL,
    message BYTEA NOT NULL
);

CREATE INDEX ON deferred_rabbitmq_messages(execute_time);