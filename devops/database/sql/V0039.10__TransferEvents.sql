CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_characters(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
BEGIN
    INSERT INTO wow_match_view_character_presence (
        view_id,
        unit_guid,
        unit_name,
        flags,
        has_combatant_info
    )
    SELECT DISTINCT ON(source->>'guid')
        in_match_view_uuid,
        source->>'guid',
        source->>'name',
        (source->>'flags')::BIGINT,
        FALSE
    FROM wow_combat_log_events
    WHERE combat_log_uuid = in_combat_log_uuid
        AND tm >= in_start
        AND tm <= in_end
        AND source IS NOT NULL
    ORDER BY source->>'guid', tm DESC
    ON CONFLICT (view_id, unit_guid) DO UPDATE
        SET flags = EXCLUDED.flags;

    INSERT INTO wow_match_view_character_presence (
        view_id,
        unit_guid,
        unit_name,
        flags,
        has_combatant_info
    )
    SELECT DISTINCT ON(dest->>'guid')
        in_match_view_uuid,
        dest->>'guid',
        dest->>'name',
        (dest->>'flags')::BIGINT,
        FALSE
    FROM wow_combat_log_events
    WHERE combat_log_uuid = in_combat_log_uuid
        AND tm >= in_start
        AND tm <= in_end
        AND dest IS NOT NULL
    ORDER BY dest->>'guid', tm DESC
    ON CONFLICT (view_id, unit_guid) DO UPDATE
        SET flags = EXCLUDED.flags;

    UPDATE wow_match_view_character_presence
    SET owner_guid = sub.owner_guid
    FROM (
        SELECT *
        FROM wow_combatlog_unit_ownership
        WHERE combat_log_uuid = in_combat_log_uuid
    ) AS sub
    WHERE wow_match_view_character_presence.view_id = in_match_view_uuid
        AND wow_match_view_character_presence.unit_guid = sub.unit_guid;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_combatant_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    combatant_character_id BIGINT;
    combatant_name VARCHAR;
    new_event_id BIGINT;
    item_idx INTEGER;
    item JSONB;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'CombatantInfo'
    LOOP
        INSERT INTO wow_match_view_character_presence (
            view_id,
            unit_guid,
            flags,
            has_combatant_info
        )
        VALUES (
            in_match_view_uuid,
            tmp.evt->>'guid',
            0,
            TRUE
        )
        ON CONFLICT (view_id, unit_guid) DO UPDATE
            SET has_combatant_info = EXCLUDED.has_combatant_info
        RETURNING character_id, unit_name
        INTO combatant_character_id, combatant_name;

        INSERT INTO wow_match_view_events (
            view_id,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_combatants (
            event_id,
            character_id,
            team,
            spec_id
        )
        VALUES (
            new_event_id,
            combatant_character_id,
            COALESCE((tmp.evt->>'team')::INTEGER, 0),
            (tmp.evt->>'spec_id')::INTEGER
        );

        item_idx := 0;

        FOR item IN SELECT * FROM jsonb_array_elements(tmp.evt#>'{items}')
        LOOP
            INSERT INTO wow_match_view_combatant_items (
                event_id,
                character_id,
                idx,
                item_id,
                ilvl
            )
            VALUES (
                new_event_id,
                combatant_character_id,
                item_idx,
                (item->>'item_id')::BIGINT,
                (item->>'ilvl')::INTEGER
            );

            item_idx := item_idx + 1;
        END LOOP;

        INSERT INTO wow_user_character_cache (
            user_id,
            unit_guid,
            event_id,
            cache_time
        )
        SELECT wcl.user_id, tmp.evt->>'guid', new_event_id, tmp.tm
        FROM wow_combat_logs AS wcl
        INNER JOIN wow_user_character_association AS wuca
            ON wuca.user_id = wcl.user_id
        WHERE wcl.uuid = in_combat_log_uuid
            AND wuca.guid = tmp.evt->>'guid'
        ON CONFLICT (user_id, unit_guid) DO UPDATE
            SET event_id = (CASE WHEN (EXCLUDED.cache_time >= wow_user_character_cache.cache_time) THEN EXCLUDED.event_id
                                                                                                  ELSE wow_user_character_cache.event_id
                           END),
                cache_time = (CASE WHEN (EXCLUDED.cache_time >= wow_user_character_cache.cache_time) THEN EXCLUDED.cache_time
                                                                                                    ELSE wow_user_character_cache.cache_time
                             END);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_damage_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
    source_char_id BIGINT;
    dest_char_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'DamageDone'
    LOOP
        SELECT character_id INTO source_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.source->>'guid';

        SELECT character_id INTO dest_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.dest->>'guid';

        INSERT INTO wow_match_view_events (
            view_id,
            source_char,
            dest_char,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            source_char_id,
            dest_char_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_damage_events (
            event_id,
            spell_id,
            amount,
            overkill
        )
        VALUES (
            new_event_id,
            (tmp.evt#>>'{spell,id}')::BIGINT,
            (tmp.evt->>'amount')::INTEGER,
            (tmp.evt->>'overkill')::INTEGER
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_healing_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
    source_char_id BIGINT;
    dest_char_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'Healing'
    LOOP
        SELECT character_id INTO source_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.source->>'guid';

        SELECT character_id INTO dest_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.dest->>'guid';

        INSERT INTO wow_match_view_events (
            view_id,
            source_char,
            dest_char,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            source_char_id,
            dest_char_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_healing_events (
            event_id,
            spell_id,
            amount,
            overheal,
            absorbed
        )
        VALUES (
            new_event_id,
            (tmp.evt#>>'{spell,id}')::BIGINT,
            (tmp.evt->>'amount')::INTEGER,
            (tmp.evt->>'overheal')::INTEGER,
            (tmp.evt->>'absorbed')::INTEGER
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_auras_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
    source_char_id BIGINT;
    dest_char_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'SpellAura'
    LOOP
        SELECT character_id INTO source_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.source->>'guid';

        SELECT character_id INTO dest_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.dest->>'guid';

        INSERT INTO wow_match_view_events (
            view_id,
            source_char,
            dest_char,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            source_char_id,
            dest_char_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_aura_events (
            event_id,
            spell_id,
            aura_type,
            applied
        )
        VALUES (
            new_event_id,
            (tmp.evt#>>'{spell,id}')::BIGINT,
            tmp.evt#>>'{aura_type, type}',
            (tmp.evt->>'applied')::BOOLEAN
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_summons_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
    source_char_id BIGINT;
    dest_char_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'SpellSummon'
    LOOP
        SELECT character_id INTO source_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.source->>'guid';

        SELECT character_id INTO dest_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.dest->>'guid';

        INSERT INTO wow_match_view_events (
            view_id,
            source_char,
            dest_char,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            source_char_id,
            dest_char_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_summon_events (
            event_id,
            spell_id
        )
        VALUES (
            new_event_id,
            (tmp.evt->>'id')::BIGINT
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_resurrects_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
    source_char_id BIGINT;
    dest_char_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'Resurrect'
    LOOP
        SELECT character_id INTO source_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.source->>'guid';

        SELECT character_id INTO dest_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.dest->>'guid';

        INSERT INTO wow_match_view_events (
            view_id,
            source_char,
            dest_char,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            source_char_id,
            dest_char_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_resurrect_events (
            event_id,
            spell_id
        )
        VALUES (
            new_event_id,
            (tmp.evt->>'id')::BIGINT
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_subencounter_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND (evt->>'type' = 'EncounterStart' OR evt->>'type' = 'EncounterEnd')
    LOOP
        INSERT INTO wow_match_view_events (
            view_id,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_subencounter_events (
            event_id,
            encounter_id,
            encounter_name,
            is_start
        )
        VALUES (
            new_event_id,
            (tmp.evt->>'encounter_id')::INTEGER,
            tmp.evt->>'encounter_name',
            tmp.evt->>'type' = 'EncounterStart'
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_death_events(in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    tmp RECORD;
    new_event_id BIGINT;
    dest_char_id BIGINT;
BEGIN
    FOR tmp IN
        SELECT *
        FROM wow_combat_log_events
        WHERE combat_log_uuid = in_combat_log_uuid
            AND tm >= in_start
            AND tm <= in_end
            AND evt->>'type' = 'UnitDied'
    LOOP
        SELECT character_id INTO dest_char_id
        FROM wow_match_view_character_presence
        WHERE view_id = in_match_view_uuid
            AND unit_guid = tmp.dest->>'guid';

        INSERT INTO wow_match_view_events (
            view_id,
            dest_char,
            tm,
            log_line
        )
        VALUES (
            in_match_view_alt_id,
            dest_char_id,
            tmp.tm,
            tmp.log_line
        )
        RETURNING event_id INTO new_event_id;

        INSERT INTO wow_match_view_death_events (
            event_id
        )
        VALUES (
            new_event_id
        );
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_match_combat_log_events(old_match_uuid UUID, new_match_uuid UUID, in_match_view_uuid UUID, in_match_view_alt_id BIGINT, in_combat_log_uuid UUID, in_start TIMESTAMPTZ, in_end TIMESTAMPTZ)
RETURNS VOID AS $$
DECLARE
    start_tm TIMESTAMPTZ := clock_timestamp();
BEGIN
    RAISE NOTICE 'Start Transferring % - % - %', old_match_uuid, in_match_view_uuid, in_combat_log_uuid;
    PERFORM transfer_wow_match_combat_log_characters(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_combatant_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_damage_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_healing_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_auras_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_summons_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_resurrects_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_subencounter_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);
    PERFORM transfer_wow_match_combat_log_death_events(in_match_view_uuid, in_match_view_alt_id, in_combat_log_uuid, in_start, in_end);

    UPDATE vods
    SET match_uuid = new_match_uuid
    WHERE match_uuid = old_match_uuid;
    
    RAISE NOTICE 'Finish Transferring % - % - % [%]', old_match_uuid, in_match_view_uuid, in_combat_log_uuid, clock_timestamp() - start_tm;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_arenas(in_match_uuid UUID)
RETURNS VOID AS $$
DECLARE 
    new_match_uuid UUID;
    new_view_uuid UUID;
    new_view_alt_id BIGINT;
    tmp RECORD;
BEGIN
    INSERT INTO matches (
        uuid
    )
    VALUES (
        gen_random_uuid()
    )
    RETURNING uuid INTO new_match_uuid;

    INSERT INTO new_wow_arenas (
        match_uuid,
        tr,
        combatants_key,
        instance_id,
        arena_type
    )
    SELECT
        new_match_uuid,
        tstzrange(wa.tm, wa.finish_time, '[]'),
        wa.combatants_key,
        wa.instance_id,
        wa.arena_type
    FROM wow_arenas AS wa
    WHERE wa.match_uuid = in_match_uuid AND wa.finish_time IS NOT NULL AND wa.combatants_key != '';

    FOR tmp IN
        SELECT wcl.user_id, wa.*, wcl.uuid AS "combat_log_uuid", wcl.combat_log_version, wcl.advanced_log, wcl.build_version
        FROM wow_arenas AS wa
        INNER JOIN wow_match_combat_log_association AS wcla
            ON wcla.match_uuid = wa.match_uuid
        INNER JOIN wow_combat_logs AS wcl
            ON wcl.uuid = wcla.combat_log_uuid
        WHERE wa.match_uuid = in_match_uuid AND wa.finish_time IS NOT NULL AND wa.combatants_key != ''
    LOOP
        INSERT INTO wow_match_view (
            id,
            user_id,
            start_tm,
            end_tm,
            match_uuid,
            combat_log_version,
            advanced_log,
            build_version
        )
        VALUES (
            gen_random_uuid(),
            tmp.user_id,
            tmp.tm,
            tmp.finish_time,
            new_match_uuid,
            tmp.combat_log_version,
            tmp.advanced_log,
            tmp.build_version
        )
        RETURNING id, alt_id INTO new_view_uuid, new_view_alt_id;

        INSERT INTO wow_arena_view (
            view_id,
            instance_id,
            arena_type,
            winning_team_id,
            match_duration_seconds,
            new_ratings            
        )
        VALUES (
            new_view_uuid,
            tmp.instance_id,
            tmp.arena_type,
            tmp.winning_team_id,
            tmp.match_duration_seconds,
            tmp.new_ratings
        );

        PERFORM transfer_wow_match_combat_log_events(tmp.match_uuid, new_match_uuid, new_view_uuid, new_view_alt_id, tmp.combat_log_uuid, tmp.tm, tmp.finish_time);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_challenges(in_match_uuid UUID)
RETURNS VOID AS $$
DECLARE 
    new_match_uuid UUID;
    new_view_uuid UUID;
    new_view_alt_id BIGINT;
    tmp RECORD;
BEGIN
    INSERT INTO matches (
        uuid
    )
    VALUES (
        gen_random_uuid()
    )
    RETURNING uuid INTO new_match_uuid;

    INSERT INTO new_wow_challenges (
        match_uuid,
        tr,
        combatants_key,
        instance_id,
        keystone_level
    )
    SELECT
        new_match_uuid,
        tstzrange(wc.tm, wc.finish_time, '[]'),
        wc.combatants_key,
        wc.instance_id,
        wc.keystone_level
    FROM wow_challenges AS wc
    WHERE wc.match_uuid = in_match_uuid AND wc.finish_time IS NOT NULL AND wc.combatants_key != '';

    FOR tmp IN
        SELECT wcl.user_id, wc.*, wcl.uuid AS "combat_log_uuid", wcl.combat_log_version, wcl.advanced_log, wcl.build_version
        FROM wow_challenges AS wc
        INNER JOIN wow_match_combat_log_association AS wcla
            ON wcla.match_uuid = wc.match_uuid
        INNER JOIN wow_combat_logs AS wcl
            ON wcl.uuid = wcla.combat_log_uuid
        WHERE wc.match_uuid = in_match_uuid AND wc.finish_time IS NOT NULL AND wc.combatants_key != ''
    LOOP
        INSERT INTO wow_match_view (
            id,
            user_id,
            start_tm,
            end_tm,
            match_uuid,
            combat_log_version,
            advanced_log,
            build_version
        )
        VALUES (
            gen_random_uuid(),
            tmp.user_id,
            tmp.tm,
            tmp.finish_time,
            new_match_uuid,
            tmp.combat_log_version,
            tmp.advanced_log,
            tmp.build_version
        )
        RETURNING id, alt_id INTO new_view_uuid, new_view_alt_id;

        INSERT INTO wow_challenge_view (
            view_id,
            challenge_name,
            instance_id,
            keystone_level,
            time_ms,
            success
        )
        VALUES (
            new_view_uuid,
            tmp.challenge_name,
            tmp.instance_id,
            tmp.keystone_level,
            tmp.time_ms,
            tmp.success
        );

        PERFORM transfer_wow_match_combat_log_events(tmp.match_uuid, new_match_uuid, new_view_uuid, new_view_alt_id, tmp.combat_log_uuid, tmp.tm, tmp.finish_time);
    END LOOP;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION transfer_wow_encounters(in_match_uuid UUID)
RETURNS VOID AS $$
DECLARE 
    new_match_uuid UUID;
    new_view_uuid UUID;
    new_view_alt_id BIGINT;
    tmp RECORD;
BEGIN
    INSERT INTO matches (
        uuid
    )
    VALUES (
        gen_random_uuid()
    )
    RETURNING uuid INTO new_match_uuid;

    INSERT INTO new_wow_encounters (
        match_uuid,
        tr,
        combatants_key,
        encounter_id,
        difficulty,
        instance_id
    )
    SELECT
        new_match_uuid,
        tstzrange(we.tm, we.finish_time, '[]'),
        we.combatants_key,
        we.encounter_id,
        we.difficulty,
        we.instance_id
    FROM wow_encounters AS we
    WHERE we.match_uuid = in_match_uuid AND we.finish_time IS NOT NULL AND we.combatants_key != '';

    FOR tmp IN
        SELECT wcl.user_id, we.*, wcl.uuid AS "combat_log_uuid", wcl.combat_log_version, wcl.advanced_log, wcl.build_version
        FROM wow_encounters AS we
        INNER JOIN wow_match_combat_log_association AS wcla
            ON wcla.match_uuid = we.match_uuid
        INNER JOIN wow_combat_logs AS wcl
            ON wcl.uuid = wcla.combat_log_uuid
        WHERE we.match_uuid = in_match_uuid AND we.finish_time IS NOT NULL AND we.combatants_key != ''
    LOOP
        INSERT INTO wow_match_view (
            id,
            user_id,
            start_tm,
            end_tm,
            match_uuid,
            combat_log_version,
            advanced_log,
            build_version
        )
        VALUES (
            gen_random_uuid(),
            tmp.user_id,
            tmp.tm,
            tmp.finish_time,
            new_match_uuid,
            tmp.combat_log_version,
            tmp.advanced_log,
            tmp.build_version
        )
        RETURNING id, alt_id INTO new_view_uuid, new_view_alt_id;

        INSERT INTO wow_encounter_view (
            view_id,
            encounter_id,
            encounter_name,
            difficulty,
            num_players,
            instance_id,
            success     
        )
        VALUES (
            new_view_uuid,
            tmp.encounter_id,
            tmp.encounter_name,
            tmp.difficulty,
            tmp.num_players,
            tmp.instance_id,
            tmp.success
        );

        PERFORM transfer_wow_match_combat_log_events(tmp.match_uuid, new_match_uuid, new_view_uuid, new_view_alt_id, tmp.combat_log_uuid, tmp.tm, tmp.finish_time);
    END LOOP;
END;
$$ LANGUAGE plpgsql;