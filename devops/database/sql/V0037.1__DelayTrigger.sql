CREATE OR REPLACE FUNCTION trigger_deferred_rabbitmq_messages()
    RETURNS trigger AS
$$
    BEGIN
        PERFORM pg_notify(
            'rabbitmq_delay',
            jsonb_build_object(
                'id', NEW.id,
                'execute_time', NEW.execute_time
            ) #>> '{}');
        RETURN NEW;
    END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS deferred_rabbitmq_messages_notification ON deferred_rabbitmq_messages;
CREATE TRIGGER deferred_rabbitmq_messages_notification
    AFTER INSERT ON deferred_rabbitmq_messages
    FOR EACH ROW
    EXECUTE FUNCTION trigger_deferred_rabbitmq_messages();
