
CREATE OR REPLACE FUNCTION trigger_new_user_feature_flags()
    RETURNS trigger AS
$$
BEGIN
    INSERT INTO squadov.user_feature_flags (user_id)
    VALUES (NEW.id);

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;