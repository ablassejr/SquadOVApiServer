use crate::{
    SquadOvError,
    riot::games::{
        TftPlayerMatchSummary,
        TftCompanionDto,
        TftUnitDto,
        TftTraitDto,
    }
};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

pub async fn list_tft_match_summaries_for_puuid(ex: &PgPool, puuid: &str, start: i64, end: i64) -> Result<Vec<TftPlayerMatchSummary>, SquadOvError> {
    let mut match_summaries: Vec<TftPlayerMatchSummary> = sqlx::query!(
        r#"
        SELECT
            tmi.match_uuid AS "match_uuid!",
            tmi.game_datetime AS "game_datetime!",
            tmi.game_length as "game_length!",
            tmi.game_variation AS "game_variation",
            tmi.game_version AS "game_version!",
            tmi.queue_id AS "queue_id!",
            tmi.tft_set_number AS "tft_set_number!",
            tmp.companion_content_id AS "companion_content_id!",
            tmp.companion_skin_id AS "companion_skin_id!",
            tmp.companion_species AS "companion_species!",
            tmp.level AS "level!",
            tmp.placement AS "placement!",
            tmp.last_round AS "last_round!",
            (vod.video_uuid IS NOT NULL) AS "has_vod!"
        FROM squadov.tft_match_info AS tmi
        INNER JOIN squadov.tft_match_participants AS tmp
            ON tmp.match_uuid = tmi.match_uuid
        INNER JOIN squadov.riot_account_links AS ral
            ON ral.puuid = tmp.puuid
        INNER JOIN squadov.users AS u
            ON u.id = ral.user_id
        LEFT JOIN squadov.vods AS vod
            ON vod.match_uuid = tmi.match_uuid
                AND vod.user_uuid = u.uuid
        WHERE tmp.puuid = $1
            AND tmi.tft_set_number >= 3
        ORDER BY tmi.game_datetime DESC
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
            TftPlayerMatchSummary {
                match_uuid: x.match_uuid,
                game_datetime: x.game_datetime,
                game_length: x.game_length,
                game_variation: x.game_variation,
                game_version: x.game_version,
                queue_id: x.queue_id,
                tft_set_number: x.tft_set_number,
                companion: TftCompanionDto{
                    content_id: x.companion_content_id,
                    skin_id: x.companion_skin_id,
                    species: x.companion_species,
                },
                level: x.level,
                placement: x.placement,
                last_round: x.last_round,
                traits: vec![],
                units: vec![],
                has_vod: x.has_vod,
            }
        })
        .collect();
    let match_uuids: Vec<Uuid> = match_summaries.iter().map(|x| { x.match_uuid.clone() }).collect();

    // These need to be separate queries because SQLX doesn't really let us
    // do a ARRAY_AGG cleanly (i think?) so this is easier + type safe.
    let mut unit_map: HashMap<Uuid, Vec<TftUnitDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT *
        FROM squadov.tft_match_participant_units
        WHERE match_uuid = ANY($1)
            AND puuid = $2
        ",
        &match_uuids,
        puuid,
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .for_each(|x| {
            let match_uuid = x.match_uuid.clone();
            if !unit_map.contains_key(&match_uuid) {
                unit_map.insert(match_uuid.clone(), Vec::new());
            }
            let vec = unit_map.get_mut(&match_uuid).unwrap();
            vec.push(TftUnitDto{
                items: x.items,
                character_id: x.character_id,
                chosen: x.chosen,
                name: x.name,
                rarity: x.rarity,
                tier: x.tier,
            });
        });

    let mut trait_map: HashMap<Uuid, Vec<TftTraitDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT *
        FROM squadov.tft_match_participant_traits
        WHERE match_uuid = ANY($1)
            AND puuid = $2
        ",
        &match_uuids,
        puuid,
    )
        .fetch_all(ex)
        .await?
        .into_iter()
        .for_each(|x| {
            let match_uuid = x.match_uuid.clone();
            if !trait_map.contains_key(&match_uuid) {
                trait_map.insert(match_uuid.clone(), Vec::new());
            }
            let vec = trait_map.get_mut(&match_uuid).unwrap();
            vec.push(TftTraitDto{
                name: x.name,
                num_units: x.num_units,
                style: x.style,
                tier_current: x.tier_current,
                tier_total: x.tier_total,
            });
        });

    for m in &mut match_summaries {
        m.units.extend(unit_map.remove(&m.match_uuid).unwrap_or(vec![]));
        m.traits.extend(trait_map.remove(&m.match_uuid).unwrap_or(vec![]));
    }

    Ok(match_summaries)
}