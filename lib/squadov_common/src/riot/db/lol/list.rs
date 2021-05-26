use crate::{
    SquadOvError,
    riot::{
        LolMatchFilters,
        games::{
            LolPlayerMatchSummary,
            LolMiniParticipantStats,
        },
    },
    matches::MatchPlayerPair,
};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

pub async fn list_lol_match_summaries_for_uuids(ex: &PgPool, uuids: &[MatchPlayerPair]) -> Result<Vec<LolPlayerMatchSummary>, SquadOvError> {
    let match_uuids = uuids.iter().map(|x| { x.match_uuid.clone() }).collect::<Vec<Uuid>>();
    let player_uuids = uuids.iter().map(|x| { x.player_uuid.clone() }).collect::<Vec<Uuid>>();

    let mut match_summaries: Vec<LolPlayerMatchSummary> = sqlx::query!(
        r#"
        SELECT
            lmi.match_uuid,
            lmi.game_creation,
            lmi.game_duration,
            lmi.game_type,
            lmi.queue_id,
            lmi.season_id,
            lmi.map_id,
            lmi.game_mode,
            lmi.game_version,
            lmpi.participant_id AS "current_participant_id!",
            (vod.video_uuid IS NOT NULL) AS "has_vod!",
            u.uuid AS "user_uuid!"
        FROM UNNEST($1::UUID[], $2::UUID[]) AS inp(match_uuid, user_uuid)
        INNER JOIN squadov.lol_match_info AS lmi
            ON lmi.match_uuid = inp.match_uuid
        INNER JOIN squadov.lol_match_participant_identities AS lmpi
            ON lmpi.match_uuid = lmi.match_uuid
        INNER JOIN squadov.riot_accounts AS ra
            ON ra.summoner_id = lmpi.summoner_id
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        INNER JOIN squadov.users AS u
            ON u.id = ral.user_id
                AND u.uuid = inp.user_uuid
        LEFT JOIN squadov.vods AS vod
            ON vod.match_uuid = lmi.match_uuid
                AND vod.user_uuid = u.uuid
                AND vod.is_clip = FALSE
        ORDER BY lmi.game_creation DESC
        "#,
        &match_uuids,
        &player_uuids,
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .map(|x| {
            LolPlayerMatchSummary {
                match_uuid: x.match_uuid,
                user_uuid: x.user_uuid,
                game_creation: x.game_creation,
                game_duration: x.game_duration,
                game_type: x.game_type,
                queue_id: x.queue_id,
                season_id: x.season_id,
                map_id: x.map_id,
                game_mode: x.game_mode,
                game_version: x.game_version,
                current_participant_id: x.current_participant_id,
                participants: vec![],
                has_vod: x.has_vod,
            }
        })
        .collect();

    let mut participant_map: HashMap<Uuid, Vec<LolMiniParticipantStats>> = HashMap::new();
    sqlx::query!(
        "
        SELECT
            lmp.match_uuid,
            lmp.participant_id,
            lmp.champion_id,
            lmpi.summoner_name,
            lmp.team_id,
            lmp.kills,
            lmp.deaths,
            lmp.assists,
            lmp.total_damage_dealt_to_champions,
            lmp.total_minions_killed,
            lmp.wards_placed,
            lmp.lane,
            lmp.win
        FROM squadov.lol_match_participants AS lmp
        LEFT JOIN squadov.lol_match_participant_identities AS lmpi
            ON lmpi.participant_id = lmp.participant_id
                AND lmpi.match_uuid = lmp.match_uuid
        LEFT JOIN squadov.riot_accounts AS ra
            ON ra.summoner_id = lmpi.summoner_id
        WHERE lmp.match_uuid = ANY($1)
        ",
        &match_uuids,
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !participant_map.contains_key(&x.match_uuid) {
                participant_map.insert(x.match_uuid.clone(), Vec::new());
            }
            let vec = participant_map.get_mut(&x.match_uuid).unwrap();
            vec.push(LolMiniParticipantStats{
                participant_id: x.participant_id,
                champion_id: x.champion_id,
                summoner_name: x.summoner_name,
                team_id: x.team_id,
                kills: x.kills,
                deaths: x.deaths,
                assists: x.assists,
                total_damage_dealt_to_champions: x.total_damage_dealt_to_champions,
                total_minions_killed: x.total_minions_killed,
                wards_placed: x.wards_placed,
                lane: x.lane,
                win: x.win,
            });
        });

    for m in &mut match_summaries {
        m.participants.extend(participant_map.get(&m.match_uuid).cloned().unwrap_or(vec![]));
    }

    Ok(match_summaries)
}

pub async fn list_lol_match_summaries_for_puuid(ex: &PgPool, puuid: &str, user_uuid: &Uuid, start: i64, end: i64, filters: &LolMatchFilters) -> Result<Vec<LolPlayerMatchSummary>, SquadOvError> {
    let uuids: Vec<MatchPlayerPair> = sqlx::query!(
        r#"
        SELECT lmi.match_uuid
        FROM squadov.lol_match_info AS lmi
        INNER JOIN squadov.lol_match_participant_identities AS lmpi
            ON lmpi.match_uuid = lmi.match_uuid
        INNER JOIN squadov.riot_accounts AS ra
            ON ra.summoner_id = lmpi.summoner_id
        LEFT JOIN squadov.vods AS v
            ON v.match_uuid = lmi.match_uuid
                AND v.user_uuid = $4
                AND v.is_clip = FALSE
        WHERE ra.puuid = $1
            AND (CARDINALITY($5::INTEGER[]) = 0 OR lmi.map_id = ANY($5))
            AND (CARDINALITY($6::VARCHAR[]) = 0 OR lmi.game_mode = ANY($6))
            AND (NOT $7::BOOLEAN OR v.video_uuid IS NOT NULL)
        ORDER BY lmi.game_creation DESC
        LIMIT $2 OFFSET $3
        "#,
        puuid,
        end - start,
        start,
        user_uuid,
        &filters.maps.as_ref().unwrap_or(&vec![]).iter().map(|x| { *x }).collect::<Vec<i32>>(),
        &filters.modes.as_ref().unwrap_or(&vec![]).iter().map(|x| { x.clone() }).collect::<Vec<String>>(),
        filters.has_vod.unwrap_or(false),
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .map(|x| {
            MatchPlayerPair{
                match_uuid: x.match_uuid,
                player_uuid: user_uuid.clone(),
            }
        })
        .collect();
    list_lol_match_summaries_for_uuids(&*ex, &uuids).await
}

pub async fn get_participant_ids_in_lol_match_from_user_uuids(ex: &PgPool, match_uuid: &Uuid, user_uuids: &[Uuid]) -> Result<Vec<(Uuid, i32)>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT u.uuid, lmpi.participant_id
            FROM squadov.lol_match_participant_identities AS lmpi
            INNER JOIN squadov.riot_accounts AS ra
                ON ra.summoner_id = lmpi.summoner_id
            INNER JOIN squadov.riot_account_links AS ral
                ON ral.puuid = ra.puuid
            INNER JOIN squadov.users AS u
                ON u.id = ral.user_id
            WHERE lmpi.match_uuid = $1
                AND u.uuid = ANY($2)
            ",
            match_uuid,
            user_uuids,
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                (x.uuid, x.participant_id)
            })
            .collect()
    )
}