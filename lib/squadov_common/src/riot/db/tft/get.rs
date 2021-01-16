use crate::{
    SquadOvError,
    riot::games::{
        WrappedTftMatch,
        TftMatchDto,
        TftInfoDto,
        TftParticipantDto,
        TftCompanionDto,
        TftUnitDto,
        TftTraitDto,
    }
};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

pub async fn get_tft_match(ex: &PgPool, match_uuid: &Uuid) -> Result<WrappedTftMatch, SquadOvError> {
    let match_info = sqlx::query!(
        "
        SELECT *
        FROM squadov.tft_match_info
        WHERE match_uuid = $1
        ",
        match_uuid
    )
        .fetch_one(&*ex)
        .await?;

    let match_participants = sqlx::query!(
        "
        SELECT tmp.*, ra.summoner_name
        FROM squadov.tft_match_participants AS tmp
        INNER JOIN squadov.riot_accounts AS ra
            ON ra.puuid = tmp.puuid
        WHERE tmp.match_uuid = $1
        ",
        match_uuid
    )
        .fetch_all(&*ex)
        .await?;
    
    let mut unit_map: HashMap<String, Vec<TftUnitDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT *
        FROM squadov.tft_match_participant_units
        WHERE match_uuid = $1
        ",
        match_uuid
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !unit_map.contains_key(&x.puuid) {
                unit_map.insert(x.puuid.clone(), Vec::new());
            }
            let vec = unit_map.get_mut(&x.puuid).unwrap();
            vec.push(TftUnitDto{
                items: x.items,
                character_id: x.character_id,
                chosen: x.chosen,
                name: x.name,
                rarity: x.rarity,
                tier: x.tier,
            });
        });

    let mut trait_map: HashMap<String, Vec<TftTraitDto>> = HashMap::new();
    sqlx::query!(
        "
        SELECT *
        FROM squadov.tft_match_participant_traits
        WHERE match_uuid = $1
        ",
        match_uuid
    )
        .fetch_all(&*ex)
        .await?
        .into_iter()
        .for_each(|x| {
            if !trait_map.contains_key(&x.puuid) {
                trait_map.insert(x.puuid.clone(), Vec::new());
            }
            let vec = trait_map.get_mut(&x.puuid).unwrap();
            vec.push(TftTraitDto{
                name: x.name,
                num_units: x.num_units,
                style: x.style,
                tier_current: x.tier_current,
                tier_total: x.tier_total,
            });
        });

    Ok(
        WrappedTftMatch {
            data: TftMatchDto{
                info: TftInfoDto{
                    game_datetime: Some(match_info.game_datetime),
                    game_length: match_info.game_length,
                    game_variation: match_info.game_variation,
                    game_version: match_info.game_version,
                    queue_id: match_info.queue_id,
                    tft_set_number: match_info.tft_set_number,
                    participants: match_participants.iter().map(|x| {
                        TftParticipantDto {
                            gold_left: x.gold_left,
                            last_round: x.last_round,
                            level: x.level,
                            placement: x.placement,
                            players_eliminated: x.players_eliminated,
                            puuid: x.puuid.clone(),
                            time_eliminated: x.time_eliminated,
                            total_damage_to_players: x.total_damage_to_players,
                            units: unit_map.remove(&x.puuid).unwrap_or(vec![]),
                            traits: trait_map.remove(&x.puuid).unwrap_or(vec![]),
                            companion: TftCompanionDto{
                                content_id: x.companion_content_id.clone(),
                                skin_id: x.companion_skin_id,
                                species: x.companion_species.clone(),
                            },
                        }
                    }).collect(),
                },
            },
            puuid_to_name: match_participants.iter().map(|x| {
                (x.puuid.clone(), x.summoner_name.as_ref().unwrap_or(&String::new()).clone())
            }).collect()
        }
    )
}