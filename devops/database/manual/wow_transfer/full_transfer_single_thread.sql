DO $$ BEGIN
    PERFORM transfer_wow_arenas(match_uuid)
    FROM wow_arenas;

    PERFORM transfer_wow_challenges(match_uuid)
    FROM wow_challenges;

    PERFORM transfer_wow_encounters(match_uuid)
    FROM wow_encounters;
END $$ LANGUAGE plpgsql;