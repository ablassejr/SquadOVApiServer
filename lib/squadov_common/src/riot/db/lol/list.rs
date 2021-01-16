use crate::{
    SquadOvError,
    riot::games::{
        LolPlayerMatchSummary,
        LolMiniParticipantStats,
    }
};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

pub async fn list_lol_match_summaries_for_puuid(ex: &PgPool, puuid: &str, start: i64, end: i64) -> Result<Vec<LolPlayerMatchSummary>, SquadOvError> {
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
            (vod.video_uuid IS NOT NULL) AS "has_vod!"
        FROM squadov.lol_match_info AS lmi
        INNER JOIN squadov.lol_match_participant_identities AS lmpi
            ON lmpi.match_uuid = lmi.match_uuid
        INNER JOIN squadov.riot_accounts AS ra
            ON ra.summoner_id = lmpi.summoner_id
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = ra.puuid
        INNER JOIN squadov.users AS u
            ON u.id = ral.user_id
        LEFT JOIN squadov.vods AS vod
            ON vod.match_uuid = lmi.match_uuid
                AND vod.user_uuid = u.uuid
        WHERE ra.puuid = $1
        ORDER BY lmi.game_creation DESC
        LIMIT $2 OFFSET $3
        "#,
        puuid,
        end - start,
        start
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .map(|x| {
            LolPlayerMatchSummary {
                match_uuid: x.match_uuid,
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

    let match_uuids: Vec<Uuid> = match_summaries.iter().map(|x| { x.match_uuid.clone() }).collect();

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
            lmp.total_damage_dealt,
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
                total_damage_dealt: x.total_damage_dealt,
                total_minions_killed: x.total_minions_killed,
                wards_placed: x.wards_placed,
                lane: x.lane,
                win: x.win,
            });
        });

    for m in &mut match_summaries {
        m.participants.extend(participant_map.remove(&m.match_uuid).unwrap_or(vec![]));
    }

    Ok(match_summaries)
}