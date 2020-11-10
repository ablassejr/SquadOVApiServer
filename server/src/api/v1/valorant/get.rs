use squadov_common;
use crate::api;
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::sync::Arc;
use std::vec::Vec;
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct GetValorantMatchDetailsInput {
    match_id: String,
}

#[derive(Deserialize)]
pub struct GetValorantPlayerMatchMetadataInput {
    match_id: String,
    puuid: String
}

struct RawValorantMatchPlayerData {
    #[allow(dead_code)]
    match_id: String,
    team_id: String,
    puuid: String,
    character_id: String,
    competitive_tier: i32,
    total_combat_score: i32,
    rounds_played: i32,
    kills: i32,
    deaths: i32,
    assists: i32
}

impl RawValorantMatchPlayerData {
    fn cook(self) -> super::ValorantMatchPlayerData {
        super::ValorantMatchPlayerData {
            subject: self.puuid,
            character_id: self.character_id,
            competitive_tier: self.competitive_tier,
            team_id: self.team_id,
            stats: super::ValorantMatchPlayerStats{
                score: self.total_combat_score,
                rounds_played: self.rounds_played,
                kills: self.kills,
                deaths: self.deaths,
                assists: self.assists
            }
        }
    }
}

struct RawValorantMatchKillData {
    #[allow(dead_code)]
    match_id: String,
    round_num: i32,
    killer_puuid: Option<String>,
    victim_puuid: String,
    round_time: i32,
    damage_type: String,
    damage_item: String,
    is_secondary_fire: bool
}

impl RawValorantMatchKillData {
    fn cook(self) -> super::ValorantMatchKillData {
        super::ValorantMatchKillData {
            round_time: self.round_time,
            round: self.round_num,
            killer: self.killer_puuid,
            victim: self.victim_puuid,
            finishing_damage: super::ValorantMatchKillFinishingDamage{
                damage_type: self.damage_type,
                damage_item: self.damage_item,
                is_secondary_fire_mode: self.is_secondary_fire
            }
        }
    }
}

struct RawValorantMatchDamage {
    #[allow(dead_code)]
    match_id: String,
    round_num: i32,
    instigator_puuid: String,
    receiver_puuid: String,
    damage: i32,
    legshots: i32,
    bodyshots: i32,
    headshots: i32
}

struct RawValorantRoundPlayerLoadout {
    #[allow(dead_code)]
    match_id: String,
    round_num: i32,
    puuid: String,
    loadout_value: i32,
    remaining_money: i32,
    spent_money: i32,
    weapon: String,
    armor: String
}

struct RawValorantRoundPlayerStats {
    #[allow(dead_code)]
    match_id: String,
    round_num: i32,
    puuid: String,
    combat_score: i32
}

struct RawValorantRound {
    #[allow(dead_code)]
    match_id: String,
    round_num: i32,
    plant_round_time: Option<i32>,
    planter_puuid: Option<String>,
    defuse_round_time: Option<i32>,
    defuser_puuid: Option<String>,
    team_round_winner: String
}

impl RawValorantRound {
    fn cook(self, all_damage: Vec<&RawValorantMatchDamage>, all_loadouts: Vec<&RawValorantRoundPlayerLoadout>, all_stats: Vec<&RawValorantRoundPlayerStats>) -> super::ValorantMatchRoundData {
        let player_econs: Vec<super::ValorantMatchPlayerRoundEconomyData> = all_loadouts.iter().map(|x| {
            super::ValorantMatchPlayerRoundEconomyData {
                subject: x.puuid.clone(),
                armor: x.armor.clone(),
                weapon: x.weapon.clone(),
                remaining: x.remaining_money,
                loadout_value: x.loadout_value,
                spent: x.spent_money
            }
        }).collect();

        let player_stats: Vec<super::ValorantMatchPlayerRoundStatsData> = all_stats.iter().map(|x| {
            super::ValorantMatchPlayerRoundStatsData {
                subject: x.puuid.clone(),
                score: x.combat_score,
                damage: all_damage.iter().filter(|y| y.instigator_puuid == x.puuid).map(|y| {
                    super::ValorantMatchDamageData {
                        receiver: y.receiver_puuid.clone(),
                        damage: y.damage,
                        legshots: y.legshots,
                        bodyshots: y.bodyshots,
                        headshots: y.headshots
                    }
                }).collect()
            }
        }).collect();
    
        super::ValorantMatchRoundData {
            round_num: self.round_num,
            plant_round_time: self.plant_round_time,
            bomb_planter: self.planter_puuid,
            defuse_round_time: self.defuse_round_time,
            bomb_defuser: self.defuser_puuid,
            winning_team: self.team_round_winner,
            player_stats: player_stats,
            player_economies: Some(player_econs)
        }
    }
}

struct RawValorantPlayerMatchMetadata {
    pub match_id: String,
    pub puuid: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>
}

impl api::ApiApplication {
    async fn get_valorant_match_metadata(&self, match_id: &str) -> Result<super::ValorantMatchMetadata, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            super::ValorantMatchMetadata,
            r#"
            SELECT
                match_id,
                game_mode,
                map AS "map_id",
                is_ranked,
                provisioning_flow_id,
                game_version,
                server_start_time_utc
            FROM squadov.valorant_matches
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_one(&*self.pool)
            .await?
        )
    }

    async fn get_valorant_match_teams(&self, match_id: &str) -> Result<Vec<super::ValorantMatchTeamData>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            super::ValorantMatchTeamData,
            r#"
            SELECT
                team_id,
                won,
                rounds_played,
                rounds_won
            FROM squadov.valorant_match_teams
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?
        )
    }

    async fn get_valorant_match_players(&self, match_id: &str) -> Result<Vec<super::ValorantMatchPlayerData>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            RawValorantMatchPlayerData,
            r#"
            SELECT *
            FROM squadov.valorant_match_players
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| x.cook())
            .collect()
        )
    }

    async fn get_valorant_match_damage(&self, match_id: &str) -> Result<Vec<RawValorantMatchDamage>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            RawValorantMatchDamage,
            r#"
            SELECT *
            FROM squadov.valorant_match_damage
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?
        )
    }

    async fn get_valorant_match_round_player_loadouts(&self, match_id: &str) -> Result<Vec<RawValorantRoundPlayerLoadout>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            RawValorantRoundPlayerLoadout,
            r#"
            SELECT *
            FROM squadov.valorant_match_round_player_loadout
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?
        )
    }

    async fn get_valorant_match_round_player_stats(&self, match_id: &str) -> Result<Vec<RawValorantRoundPlayerStats>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            RawValorantRoundPlayerStats,
            r#"
            SELECT *
            FROM squadov.valorant_match_round_player_stats
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?
        )
    }

    async fn get_valorant_match_rounds(&self, match_id: &str) -> Result<Vec<super::ValorantMatchRoundData>, squadov_common::SquadOvError> {
        let damage = self.get_valorant_match_damage(match_id).await?;
        let loadouts = self.get_valorant_match_round_player_loadouts(match_id).await?;
        let stats = self.get_valorant_match_round_player_stats(match_id).await?;

        let rounds = sqlx::query_as!(
            RawValorantRound,
            r#"
            SELECT *
            FROM squadov.valorant_match_rounds
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?;

        let mut ret = Vec::new();
        for rnd in rounds {
            let round_damage : Vec<&RawValorantMatchDamage> = damage.iter().filter(|x| x.round_num == rnd.round_num).collect();
            let round_loadouts: Vec<&RawValorantRoundPlayerLoadout> = loadouts.iter().filter(|x| x.round_num == rnd.round_num).collect();
            let round_stats: Vec<&RawValorantRoundPlayerStats> = stats.iter().filter(|x| x.round_num == rnd.round_num).collect();
            ret.push(rnd.cook(round_damage, round_loadouts, round_stats));
        }
        Ok(ret)
    }

    async fn get_valorant_match_kills(&self, match_id: &str) -> Result<Vec<super::ValorantMatchKillData>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            RawValorantMatchKillData,
            r#"
            SELECT *
            FROM squadov.valorant_match_kill
            WHERE match_id = $1
            "#,
            match_id,
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| x.cook())
            .collect()
        )
    }

    async fn get_valorant_match_details(&self, match_id: &str) -> Result<super::FullValorantMatchData, squadov_common::SquadOvError> {
        let mut match_data = super::FullValorantMatchData{..Default::default()};
        match_data.match_uuid = match self.check_if_valorant_match_exists(match_id).await? {
            Some(x) => x.uuid,
            None => return Err(squadov_common::SquadOvError::NotFound)
        };
        match_data.match_info = self.get_valorant_match_metadata(match_id).await?;
        match_data.teams = self.get_valorant_match_teams(match_id).await?;
        match_data.players = self.get_valorant_match_players(match_id).await?;
        match_data.rounds = self.get_valorant_match_rounds(match_id).await?;
        match_data.kills = self.get_valorant_match_kills(match_id).await?;
        Ok(match_data)
    }

    async fn get_valorant_player_match_metadata(&self, match_id: &str, puuid: &str)  -> Result<Option<super::ValorantPlayerMatchMetadata>, squadov_common::SquadOvError> {
        match sqlx::query_as!(
            RawValorantPlayerMatchMetadata,
            r#"
            SELECT *
            FROM squadov.valorant_player_match_metadata
            WHERE match_id = $1
                AND puuid = $2
            "#,
            match_id,
            puuid
        )
            .fetch_optional(&*self.pool)
            .await?
        {
            Some(x) => Ok(Some(super::ValorantPlayerMatchMetadata{
                match_id: x.match_id,
                puuid: x.puuid,
                start_time: x.start_time,
                end_time: x.end_time,
                rounds: sqlx::query_as!(
                    super::ValorantPlayerRoundMetadata,
                    r#"
                    SELECT *
                    FROM squadov.valorant_player_round_metadata
                    WHERE match_id = $1
                        AND puuid = $2
                    "#,
                    match_id,
                    puuid
                )
                    .fetch_all(&*self.pool)
                    .await?,
            })),
            None => Ok(None)
        }
    }
}

pub async fn get_valorant_match_details_handler(data : web::Path<GetValorantMatchDetailsInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let match_data = app.get_valorant_match_details(&data.match_id).await?;
    Ok(HttpResponse::Ok().json(&match_data))
}

pub async fn get_valorant_player_match_metadata_handler(data: web::Path<GetValorantPlayerMatchMetadataInput>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let metadata = match app.get_valorant_player_match_metadata(&data.match_id, &data.puuid).await? {
        Some(x) => x,
        None => return Err(squadov_common::SquadOvError::NotFound)
    };
    Ok(HttpResponse::Ok().json(&metadata))
}