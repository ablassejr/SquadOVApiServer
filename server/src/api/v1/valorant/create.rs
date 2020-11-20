use squadov_common;
use crate::api;
use actix_web::{web, HttpResponse};
use uuid::Uuid;
use sqlx::{Transaction, Executor, Postgres};
use std::sync::Arc;
use serde::{Serialize,Deserialize};
use std::collections::HashMap;
use crate::api::v1;

#[derive(Deserialize)]
pub struct InputValorantMatch {
    // Valorant unique ID
    #[serde(rename = "matchId")]
    pub match_id: String,
    // Leave "raw_data" as a string as that should be the raw data
    // pulled from the VALORANT match data. We want to keep the
    // unparsed form so we don't lose data just in case we want to
    // redo how we parse the data.
    #[serde(rename = "rawData")]
    pub raw_data: Option<String>,
    #[serde(rename = "playerData")]
    pub player_data: Option<super::ValorantPlayerMatchMetadata>
}

#[derive(Serialize)]
struct CreateValorantMatchResponse<'a> {
    #[serde(rename = "matchUuid")]
    match_uuid: &'a Uuid
}

impl api::ApiApplication {
    pub async fn check_if_valorant_match_exists(&self, match_id : &str) -> Result<Option<v1::Match>, squadov_common::SquadOvError> {
        Ok(sqlx::query_as!(
            v1::Match,
            "
            SELECT vm.match_uuid as \"uuid\"
            FROM squadov.valorant_matches AS vm
            WHERE vm.match_id = $1
            ",
            match_id
        )
            .fetch_optional(&*self.pool)
            .await?)
    }

    async fn insert_valorant_match_teams(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, team_data: &Vec<super::ValorantMatchTeamData>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_teams (
                match_id,
                team_id,
                won,
                rounds_won,
                rounds_played
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in team_data.iter().enumerate() {
            sql.push(format!("(
                '{match_id}',
                '{team_id}',
                {won},
                {rounds_won},
                {rounds_played}
            )",
                match_id=match_id,
                team_id=&m.team_id,
                won=squadov_common::sql_format_bool(m.won),
                rounds_won=m.rounds_won,
                rounds_played=m.rounds_played
            ));

            if idx != team_data.len() - 1 {
                sql.push(String::from(","));
            }

            added += 1;
        }
        sql.push(String::from(" ON CONFLICT DO NOTHING"));

        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_match_players(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, player_data: &Vec<super::ValorantMatchPlayerData>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_players (
                match_id,
                team_id,
                puuid,
                character_id,
                competitive_tier,
                total_combat_score,
                rounds_played,
                kills,
                deaths,
                assists
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in player_data.iter().enumerate() {
            sql.push(format!("(
                '{match_id}',
                '{team_id}',
                '{puuid}',
                '{character_id}',
                {competitive_tier},
                {total_combat_score},
                {rounds_played},
                {kills},
                {deaths},
                {assists}
            )",
                match_id=match_id,
                team_id=&m.team_id,
                puuid=&m.subject,
                character_id=&m.character_id,
                competitive_tier=m.competitive_tier,
                total_combat_score=m.stats.score,
                rounds_played=m.stats.rounds_played,
                kills=m.stats.kills,
                deaths=m.stats.deaths,
                assists=m.stats.assists
            ));

            if idx != player_data.len() - 1 {
                sql.push(String::from(","));
            }

            added += 1;
        }
        sql.push(String::from(" ON CONFLICT DO NOTHING"));

        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_match_kills(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, kill_data: &Vec<super::ValorantMatchKillData>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_kill (
                match_id,
                round_num,
                killer_puuid,
                victim_puuid,
                round_time,
                damage_type,
                damage_item,
                is_secondary_fire
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in kill_data.iter().enumerate() {
            sql.push(format!("(
                '{match_id}',
                {round_num},
                {killer_puuid},
                '{victim_puuid}',
                {round_time},
                '{damage_type}',
                '{damage_item}',
                {is_secondary_fire}
            )",
                match_id=match_id,
                round_num=m.round,
                killer_puuid=squadov_common::sql_format_option_string(&m.killer),
                victim_puuid=&m.victim,
                round_time=m.round_time,
                damage_type=m.finishing_damage.damage_type,
                damage_item=m.finishing_damage.damage_item,
                is_secondary_fire=m.finishing_damage.is_secondary_fire_mode
            ));

            if idx != kill_data.len() - 1 {
                sql.push(String::from(","));
            }

            added += 1;
        }
        sql.push(String::from(" ON CONFLICT DO NOTHING"));

        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_match_damage<'a>(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, damage: &super::ValorantAllPlayerDamageData<'a>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_damage (
                match_id,
                round_num,
                instigator_puuid,
                receiver_puuid,
                damage,
                legshots,
                bodyshots,
                headshots
            )
            VALUES
        "));

        let mut added = 0;
        for (round_num, per_player) in damage {
            for (puuid, data) in per_player {
                for dmg in *data {
                    sql.push(format!("(
                        '{match_id}',
                        {round_num},
                        '{instigator_puuid}',
                        '{receiver_puuid}',
                        {damage},
                        {legshots},
                        {bodyshots},
                        {headshots}
                    )",
                        match_id=match_id,
                        round_num=round_num,
                        instigator_puuid=&puuid,
                        receiver_puuid=&dmg.receiver,
                        damage=dmg.damage,
                        legshots=dmg.legshots,
                        bodyshots=dmg.bodyshots,
                        headshots=dmg.headshots
                    ));

                    sql.push(String::from(","));
                    added += 1;
                }
            }
        }

        // This is responsible for removing the trailing comma.
        sql.truncate(sql.len() - 1);
        sql.push(String::from(" ON CONFLICT DO NOTHING"));

        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_match_round_player_stats<'a>(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, player_stats: &super::ValorantAllPlayerRoundStatsData<'a>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_round_player_stats (
                match_id,
                round_num,
                puuid,
                combat_score
            )
            VALUES
        "));

        let mut added = 0;
        for (round_num, round_stats) in player_stats {
            for stat in *round_stats {
                sql.push(format!("(
                    '{match_id}',
                    {round_num},
                    '{puuid}',
                    {combat_score}
                )",
                    match_id=match_id,
                    round_num=round_num,
                    puuid=&stat.subject,
                    combat_score=stat.score
                ));

                sql.push(String::from(","));
                added += 1;
            }
        }

        sql.truncate(sql.len() - 1);
        sql.push(String::from(" ON CONFLICT DO NOTHING"));

        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_match_round_player_economies<'a>(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, player_econs: &super::ValorantAllPlayerRoundEconomyData<'a>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_round_player_loadout (
                match_id,
                round_num,
                puuid,
                loadout_value,
                remaining_money,
                spent_money,
                weapon,
                armor
            )
            VALUES
        "));

        let mut added = 0;
        for (round_num, round_econs) in player_econs {
            for econ in *round_econs {
                sql.push(format!("(
                    '{match_id}',
                    {round_num},
                    '{puuid}',
                    {loadout_value},
                    {remaining_money},
                    {spent_money},
                    '{weapon}',
                    '{armor}'
                )",
                    match_id=match_id,
                    round_num=round_num,
                    puuid=&econ.subject,
                    loadout_value=econ.loadout_value,
                    remaining_money=econ.remaining,
                    spent_money=econ.spent,
                    weapon=econ.weapon,
                    armor=econ.armor
                ));

                sql.push(String::from(","));
                added += 1;
            }
        }

        if added > 0 {        
            sql.truncate(sql.len() - 1);
            sql.push(String::from(" ON CONFLICT DO NOTHING"));
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_match_rounds(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, round_data: &Vec<super::ValorantMatchRoundData>) -> Result<(), squadov_common::SquadOvError> {
        // Batch the per-round player stats and per-round player economies
        // so we can insert them all in one go.
        let mut all_player_round_stats : super::ValorantAllPlayerRoundStatsData = HashMap::new();
        let mut all_player_round_econs : super::ValorantAllPlayerRoundEconomyData = HashMap::new();
        let mut all_player_damage : super::ValorantAllPlayerDamageData = HashMap::new();

        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_match_rounds (
                match_id,
                round_num,
                plant_round_time,
                planter_puuid,
                defuse_round_time,
                defuser_puuid,
                team_round_winner
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in round_data.iter().enumerate() {
            sql.push(format!("(
                '{match_id}',
                {round_num},
                {plant_round_time},
                {planter_puuid},
                {defuse_round_time},
                {defuser_puuid},
                '{team_round_winner}'
            )",
                match_id=match_id,
                round_num=m.round_num,
                plant_round_time=squadov_common::sql_format_option_value(&m.plant_round_time),
                planter_puuid=squadov_common::sql_format_option_string(&m.bomb_planter),
                defuse_round_time=squadov_common::sql_format_option_value(&m.defuse_round_time),
                defuser_puuid=squadov_common::sql_format_option_string(&m.bomb_defuser),
                team_round_winner=&m.winning_team
            ));

            if idx != round_data.len() - 1 {
                sql.push(String::from(","));
            }

            let mut round_damage : super::ValorantPerRoundPlayerDamageData = HashMap::new();
            for ps in &m.player_stats {
                round_damage.insert(
                    ps.subject.clone(),
                    &ps.damage,
                );
            }
            all_player_round_stats.insert(m.round_num, &m.player_stats);
            
            if m.player_economies.is_some() {
                all_player_round_econs.insert(m.round_num, m.player_economies.as_ref().unwrap());
            }
            all_player_damage.insert(m.round_num, round_damage);
            added += 1;
        }
        sql.push(String::from(" ON CONFLICT DO NOTHING"));

        if added > 0 {
            tx.execute(sqlx::query(&sql.join(""))).await?;

            self.insert_valorant_match_round_player_stats(tx, match_id, &all_player_round_stats).await?;
            self.insert_valorant_match_round_player_economies(tx, match_id, &all_player_round_econs).await?;
            self.insert_valorant_match_damage(tx, match_id, &all_player_damage).await?;
        }
        Ok(())
    }

    async fn insert_valorant_raw_data(&self, tx : &mut Transaction<'_, Postgres>, match_id: &str, uuid: &Uuid, raw_data: &Option<String>) -> Result<(), squadov_common::SquadOvError> {
        match raw_data {
            Some(json) => {
                let full_match_data : super::FullValorantMatchData = serde_json::from_str(&json)?;
                let match_info = &full_match_data.match_info;
                
                if match_id != match_info.match_id {
                    return Err(squadov_common::SquadOvError::BadRequest);
                }
                
                tx.execute(
                    sqlx::query!(
                        "
                        INSERT INTO squadov.valorant_matches (
                            match_id,
                            match_uuid,
                            game_mode,
                            map,
                            is_ranked,
                            provisioning_flow_id,
                            game_version,
                            server_start_time_utc,
                            raw_data
                        )
                        VALUES (
                            $1,
                            $2,
                            $3,
                            $4,
                            $5,
                            $6,
                            $7,
                            $8,
                            $9
                        )
                        ON CONFLICT (match_id) DO UPDATE
                            SET game_mode = EXCLUDED.game_mode,
                                map = EXCLUDED.map,
                                is_ranked = EXCLUDED.is_ranked,
                                provisioning_flow_id = EXCLUDED.provisioning_flow_id,
                                game_version = EXCLUDED.game_version,
                                server_start_time_utc = EXCLUDED.server_start_time_utc,
                                raw_data = EXCLUDED.raw_data
                        ",
                        match_id,
                        uuid,
                        match_info.game_mode,
                        match_info.map_id,
                        match_info.is_ranked,
                        match_info.provisioning_flow_id,
                        match_info.game_version,
                        match_info.server_start_time_utc,
                        serde_json::from_str::<serde_json::Value>(&json)?
                    )
                ).await?;

                self.insert_valorant_match_teams(tx, match_id, &full_match_data.teams).await?;
                self.insert_valorant_match_players(tx, match_id, &full_match_data.players).await?;
                self.insert_valorant_match_rounds(tx, match_id, &full_match_data.rounds).await?;
                self.insert_valorant_match_kills(tx, match_id, &full_match_data.kills).await?;
            }
            None => {
                sqlx::query!(
                    "
                    INSERT INTO squadov.valorant_matches ( match_id, match_uuid )
                    VALUES ($1, $2)
                    ON CONFLICT DO NOTHING
                    ",
                    match_id,
                    uuid
                )
                    .execute(tx)
                    .await?;
            }
        };
        
        Ok(())
    }

    async fn insert_valorant_player_round_data(&self, tx : &mut Transaction<'_, Postgres>, data: &Vec<super::ValorantPlayerRoundMetadata>) -> Result<(), squadov_common::SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("
            INSERT INTO squadov.valorant_player_round_metadata (
                match_id,
                puuid,
                round,
                buy_time,
                round_time
            )
            VALUES
        "));

        let mut added = 0;
        for (idx, m) in data.iter().enumerate() {
            sql.push(format!("(
                '{match_id}',
                '{puuid}',
                {round},
                {buy_time},
                {round_time}
            )",
                match_id=&m.match_id,
                puuid=&m.puuid,
                round=m.round,
                buy_time=squadov_common::sql_format_option_some_time(m.buy_time.as_ref()),
                round_time=squadov_common::sql_format_option_some_time(m.round_time.as_ref())
            ));

            if idx != data.len() - 1 {
                sql.push(String::from(","));
            }

            added += 1;
        }

        if added > 0 {
            sqlx::query(&sql.join("")).execute(tx).await?;
        }
        Ok(())
    }

    async fn insert_valorant_player_data(&self, tx : &mut Transaction<'_, Postgres>, player_data: &Option<super::ValorantPlayerMatchMetadata>) -> Result<(), squadov_common::SquadOvError> {
        match player_data {
            Some(data) => {
                tx.execute(
                    sqlx::query!(
                        "
                        INSERT INTO squadov.valorant_player_match_metadata (
                            match_id,
                            puuid,
                            start_time,
                            end_time
                        )
                        VALUES (
                            $1,
                            $2,
                            $3,
                            $4
                        )
                        ",
                        &data.match_id,
                        &data.puuid,
                        &data.start_time,
                        &data.end_time
                    )
                ).await?;

                self.insert_valorant_player_round_data(tx, &data.rounds).await?;
            }
            None => ()
        };

        Ok(())
    }

    // TODO: When/if we get a production API key we need to have the user enter in the match UUID
    // and pull the data ourselves.
    pub async fn insert_valorant_match(&self, tx : &mut Transaction<'_, Postgres>, uuid: &Uuid, raw_match : InputValorantMatch) -> Result<(), squadov_common::SquadOvError> {
        self.insert_valorant_raw_data(tx, &raw_match.match_id, uuid, &raw_match.raw_data).await?;
        self.insert_valorant_player_data(tx, &raw_match.player_data).await?;
        Ok(())
    }
}

pub async fn create_new_valorant_match_handler(data : web::Json<InputValorantMatch>, app : web::Data<Arc<api::ApiApplication>>) -> Result<HttpResponse, squadov_common::SquadOvError> {
    let raw_data = data.into_inner();

    let mut tx = app.pool.begin().await?;
    // Create a new match ID and then create the match.
    // Note that we only create a new match if it's needed because
    // we could be doing a backfill.
    let internal_match = match app.check_if_valorant_match_exists(&raw_data.match_id).await? {
        Some(x) => x,
        None => app.create_new_match(&mut tx).await?
    };
    
    app.insert_valorant_match(&mut tx, &internal_match.uuid, raw_data).await?;
    tx.commit().await?;

    return Ok(HttpResponse::Ok().json(
        &CreateValorantMatchResponse{
            match_uuid: &internal_match.uuid
        }
    ))
}