use crate::{
    SquadOvError,
    riot::games::valorant::{
        ValorantMatchDto,
        ValorantMatchInfoDto,
        ValorantMatchPlayerDto,
        ValorantMatchTeamDto,
        ValorantMatchRoundResultDto,
        ValorantMatchPlayerStatsDto,
        ValorantMatchPlayerRoundStatsDto,
        ValorantMatchKillDto,
        ValorantMatchFinishingDamageDto,
        ValorantMatchDamageDto,
        ValorantMatchEconomyDto,
        FlatValorantMatchKillDto,
        FlatValorantMatchDamageDto,
        FlatValorantMatchEconomyDto,
        FlatValorantMatchPlayerRoundStatsDto,
    }
};
use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;

async fn get_valorant_match_info_dto(ex: &PgPool, match_uuid: &Uuid) -> Result<ValorantMatchInfoDto, SquadOvError> {
    Ok(
        sqlx::query_as!(
            ValorantMatchInfoDto,
            "
            SELECT
                vmul.match_id,
                vm.game_mode,
                vm.map_id,
                vm.is_ranked,
                vm.provisioning_flow_id,
                vm.server_start_time_utc,
                vm.game_length_millis,
                vm.season_id
            FROM squadov.valorant_matches AS vm
            INNER JOIN squadov.valorant_match_uuid_link AS vmul
                ON vmul.match_uuid = vm.match_uuid
            WHERE vm.match_uuid = $1
            ",
            match_uuid
        )
            .fetch_one(ex)
            .await?
    )
}

async fn get_valorant_match_players_dto(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<ValorantMatchPlayerDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT
                team_id,
                puuid,
                character_id,
                competitive_tier,
                total_combat_score,
                rounds_played,
                kills,
                deaths,
                assists
            FROM squadov.valorant_match_players
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                ValorantMatchPlayerDto {
                    puuid: x.puuid,
                    team_id: x.team_id,
                    character_id: Some(x.character_id),
                    competitive_tier: x.competitive_tier,
                    stats: Some(ValorantMatchPlayerStatsDto {
                        score: x.total_combat_score,
                        rounds_played: x.rounds_played,
                        kills: x.kills,
                        deaths: x.deaths,
                        assists: x.assists,
                    }),
                }
            })
            .collect()
    )
}

async fn get_valorant_match_teams_dto(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<ValorantMatchTeamDto>, SquadOvError> {
    Ok(
        sqlx::query_as!(
            ValorantMatchTeamDto,
            "
            SELECT
                team_id,
                won,
                rounds_won,
                rounds_played,
                num_points
            FROM squadov.valorant_match_teams
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(ex)
            .await?
    )
}

async fn get_valorant_match_kills(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<FlatValorantMatchKillDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT
                round_num,
                killer_puuid,
                victim_puuid,
                time_since_game_start_millis,
                time_since_round_start_millis,
                damage_type,
                damage_item,
                is_secondary_fire,
                assistants
            FROM squadov.valorant_match_kill
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| { 
                FlatValorantMatchKillDto {
                    round_num: x.round_num,
                    base: ValorantMatchKillDto {
                        time_since_game_start_millis: x.time_since_game_start_millis,
                        time_since_round_start_millis: x.time_since_round_start_millis,
                        killer: x.killer_puuid,
                        victim: x.victim_puuid,
                        assistants: x.assistants.unwrap_or(vec![]),
                        finishing_damage: ValorantMatchFinishingDamageDto{
                            damage_type: x.damage_type,
                            damage_item: x.damage_item,
                            is_secondary_fire_mode: x.is_secondary_fire,
                        },
                    },
                }
            })
            .collect()
    )
}

async fn get_valorant_match_damage(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<FlatValorantMatchDamageDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT
                round_num,
                instigator_puuid,
                receiver_puuid,
                damage,
                legshots,
                bodyshots,
                headshots
            FROM squadov.valorant_match_damage
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                FlatValorantMatchDamageDto {
                    round_num: x.round_num,
                    instigator: x.instigator_puuid,
                    base: ValorantMatchDamageDto {
                        receiver: x.receiver_puuid,
                        damage: x.damage,
                        legshots: x.legshots,
                        bodyshots: x.bodyshots,
                        headshots: x.headshots,
                    },
                }
            })
            .collect()
    )
}

async fn get_valorant_match_player_round_econ(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<FlatValorantMatchEconomyDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT
                round_num,
                puuid,
                loadout_value,
                remaining_money,
                spent_money,
                weapon,
                armor
            FROM squadov.valorant_match_round_player_loadout
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                FlatValorantMatchEconomyDto {
                    round_num: x.round_num,
                    puuid: x.puuid,
                    base: ValorantMatchEconomyDto {
                        loadout_value: x.loadout_value,
                        weapon: x.weapon,
                        armor: x.armor,
                        remaining: x.remaining_money,
                        spent: x.spent_money,
                    }
                }
            })
            .collect()
    )
}

async fn get_valorant_match_player_round_stats(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<FlatValorantMatchPlayerRoundStatsDto>, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            SELECT
                round_num,
                puuid,
                combat_score
            FROM squadov.valorant_match_round_player_stats
            WHERE match_uuid = $1
            ",
            match_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                FlatValorantMatchPlayerRoundStatsDto {
                    round_num: x.round_num,
                    puuid: x.puuid,
                    score: x.combat_score,
                }
            })
            .collect()
    )
}

async fn get_valorant_match_round_results(ex: &PgPool, match_uuid: &Uuid) -> Result<Vec<ValorantMatchRoundResultDto>, SquadOvError> {
    let base_round_info = sqlx::query!(
        "
        SELECT
            round_num,
            plant_round_time,
            planter_puuid,
            defuse_round_time,
            defuser_puuid,
            team_round_winner
        FROM squadov.valorant_match_rounds
        WHERE match_uuid = $1
        ORDER BY round_num ASC
        ",
        match_uuid
    )
        .fetch_all(ex)
        .await?;

    let kills = get_valorant_match_kills(ex, match_uuid).await?;
    let damage = get_valorant_match_damage(ex, match_uuid).await?;
    let econ = get_valorant_match_player_round_econ(ex, match_uuid).await?;
    let round_stats = get_valorant_match_player_round_stats(ex, match_uuid).await?;

    // We can revisit this implementation if we find that the filters are too slow and that
    // it'd be faster to create a HashMap. I'm going to avoid that since that's a lot of added
    // complexity for what I assume would be minimal performance gain since these arrays are all
    // probably pretty short (< 100 elements).
    let mut per_round_player_stats: HashMap<i32, Vec<ValorantMatchPlayerRoundStatsDto>> = HashMap::new();
    for r in &base_round_info {
        let rkills: Vec<&FlatValorantMatchKillDto> = kills.iter().filter(|x| { x.round_num == r.round_num}).collect();
        let rdamage: Vec<&FlatValorantMatchDamageDto> = damage.iter().filter(|x| { x.round_num == r.round_num}).collect();
        let recon: Vec<&FlatValorantMatchEconomyDto> = econ.iter().filter(|x| { x.round_num == r.round_num}).collect();
        let rstats: Vec<&FlatValorantMatchPlayerRoundStatsDto> = round_stats.iter().filter(|x| { x.round_num == r.round_num}).collect();

        // recon and and rstats should have 1 element per player in the match so we can use rstats
        // to pull in one element per player to construct a vec of ValorantMatchPlayerRoundStatsDto.
        per_round_player_stats.insert(r.round_num, rstats.iter().map(|x| {
            Ok(ValorantMatchPlayerRoundStatsDto {
                puuid: x.puuid.clone(),
                kills: rkills.iter().filter(|y| { y.base.killer.as_ref().unwrap_or(&String::new()).as_str() == x.puuid }).map(|y| { y.base.clone() }).collect(),
                damage: rdamage.iter().filter(|y| { y.instigator == x.puuid }).map(|y| { y.base.clone() }).collect(),
                economy: recon.iter().filter(|y| { y.puuid == x.puuid }).next().ok_or(SquadOvError::NotFound)?.base.clone(),
                score: x.score,
            })
        }).collect::<Result<Vec<ValorantMatchPlayerRoundStatsDto>, SquadOvError>>()?);
    }

    Ok(
        base_round_info
            .into_iter()
            .map(|x| {
                ValorantMatchRoundResultDto {
                    round_num: x.round_num,
                    plant_round_time: x.plant_round_time,
                    bomb_planter: x.planter_puuid,
                    defuse_round_time: x.defuse_round_time,
                    bomb_defuser: x.defuser_puuid,
                    winning_team: x.team_round_winner,
                    player_stats: per_round_player_stats.remove(&x.round_num).unwrap_or(vec![]),
                }
            })
            .collect()
    )
}

pub async fn get_valorant_match(ex: &PgPool, match_uuid: &Uuid) -> Result<ValorantMatchDto, SquadOvError> {
    Ok(
        ValorantMatchDto{
            match_info: get_valorant_match_info_dto(ex, match_uuid).await?,
            players: get_valorant_match_players_dto(ex, match_uuid).await?,
            teams: get_valorant_match_teams_dto(ex, match_uuid).await?,
            round_results: get_valorant_match_round_results(ex, match_uuid).await?,
        }
    )
}