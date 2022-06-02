use squadov_common::{
    SquadOvError,
    SquadOvGames,
    WoWEncounterStart,
    WoWChallengeStart,
    WoWArenaStart,
    WoWEncounterEnd,
    WoWChallengeEnd,
    WoWArenaEnd,
    WoWEncounter,
    WoWChallenge,
    WoWArena,
    WoWCombatantInfo,
    WoWCombatLogState,
    WowInstance,
    WowInstanceData,
    WowInstanceType,
    matches::{
        self,
        MatchPlayerPair,
    },
    wow::{
        matches as wm,
    },
    generate_combatants_key,
    generate_combatants_hashed_array,
    elastic::vod::ESVodDocument,
    vod::db as vdb,
};
use actix_web::{web, HttpResponse, HttpRequest, HttpMessage};
use crate::api;
use crate::api::auth::{SquadOvMachineId, SquadOVSession};
use crate::api::v1::{
    GenericMatchPathInput,
    RecentMatchQuery,
    RecentMatchGameQuery,
    GenericWowQuery,
};
use squadov_common::vod::VodAssociation;
use std::sync::Arc;
use uuid::Uuid;
use sqlx::{Postgres, Transaction};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use elasticsearch_dsl::{Query, BoolQuery, NestedQuery, Sort, SortOrder};

#[derive(Deserialize)]
pub struct GenericMatchCreationRequest<T> {
    pub timestamp: DateTime<Utc>,
    pub data: T,
    pub cl: WoWCombatLogState,
    pub session: Option<String>,
}

#[derive(Deserialize)]
pub struct GenericMatchFinishCreationRequest<T> {
    pub timestamp: DateTime<Utc>,
    pub data: T,
    pub combatants: Vec<WoWCombatantInfo>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct WowListQuery {
    pub has_vod: Option<bool>,
    pub encounters: Option<Vec<i32>>,
    pub raids: Option<Vec<i32>>,
    pub dungeons: Option<Vec<i32>>,
    pub arenas: Option<Vec<i32>>,
    pub brackets: Option<Vec<String>>,
    pub rating_low: Option<i32>,
    pub rating_high: Option<i32>,
    pub friendly_composition: Option<Vec<String>>,
    pub enemy_composition: Option<Vec<String>>,
    pub pov_spec: Option<Vec<i32>>,
    pub encounter_difficulties: Option<Vec<i32>>,
    pub keystone_low: Option<i32>,
    pub keystone_high: Option<i32>,
    // If not set, wins + losses. If true, only wins. If false, only losses.
    pub is_winner: Option<bool>,
    pub instance_types: Option<Vec<WowInstanceType>>,
    pub guids: Option<Vec<String>>,
    pub enabled: bool,
}

impl Default for WowListQuery {
    fn default() -> Self {
        Self {
            has_vod: None,
            encounters: None,
            raids: None,
            dungeons: None,
            arenas: None,
            brackets: None,
            rating_low: None,
            rating_high: None,
            friendly_composition: None,
            enemy_composition: None,
            pov_spec: None,
            encounter_difficulties: None,
            keystone_low: None,
            keystone_high: None,
            is_winner: None,
            instance_types: None,
            guids: None,
            enabled: true,
        }
    }
}

impl WowListQuery {
    pub fn build_es_query(&self) -> BoolQuery {
        Query::bool()
            .minimum_should_match("1")
            .should(
                Query::bool()
                    .must_not(Query::exists("data.wow"))
            )
            .should({
                let mut q = Query::bool();

                if let Some(encounters) = self.encounters.as_ref() {
                    q = q.filter(Query::terms("data.wow.encounter.encounterId", encounters.clone()));
                }

                if let Some(raids) = self.raids.as_ref() {
                    q = q.filter(Query::terms("data.wow.encounter.instanceId", raids.clone()));
                }

                if let Some(dungeons) = self.dungeons.as_ref() {
                    q = q.filter(Query::terms("data.wow.challenge.instanceId", dungeons.clone()));
                }

                if let Some(arenas) = self.arenas.as_ref() {
                    q = q.filter(Query::terms("data.wow.arena.instanceId", arenas.clone()));
                }

                let all_instances = self.all_instance_ids();
                if !all_instances.is_empty() {
                    q = q.filter(
                        Query::bool()
                            .minimum_should_match("1")
                            .should(
                                Query::bool()
                                    .must_not(Query::exists("data.wow.instance"))
                            )
                            .should(
                                Query::terms("data.wow.instance.instanceId", all_instances)
                            )
                    );
                }

                if let Some(brackets) = self.brackets.as_ref() {
                    q = q.filter(Query::terms("data.wow.arena.type", brackets.clone()));
                }

                {
                    let mut pov_query = Query::bool();
                    pov_query = pov_query.filter(Query::nested(
                        "data.wow.teams.players",
                        {
                            let mut player_query = Query::bool()
                                .filter(Query::term("data.wow.teams.players.isPov", true));

                            {
                                let mut rating_query = Query::range("data.wow.teams.players.info.data.rating");
                                if let Some(rating_low) = self.rating_low {
                                    rating_query = rating_query.gte(rating_low);
                                }

                                if let Some(rating_high) = self.rating_high {
                                    rating_query = rating_query.lte(rating_high);
                                }
                                player_query = player_query.filter(rating_query);
                            }

                            if let Some(pov_spec) = self.pov_spec.as_ref() {
                                player_query = player_query.filter(Query::terms("data.wow.teams.players.info.data.specId", pov_spec.clone()));
                            }
                            player_query
                        }
                    ));

                    q = q.filter(Query::nested(
                        "data.wow.teams",
                        {
                            let mut team_query = Query::bool()
                                .filter(pov_query);

                            if let Some(is_winner) = self.is_winner {
                                team_query = team_query.filter(Query::term("data.wow.teams.team.won", is_winner));
                            }

                            team_query
                        },
                    ));
                }
   
                if let Some(guids) = self.guids.as_ref() {
                    q = q.filter(
                        Query::nested(
                            "data.wow.teams",
                            Query::nested(
                                "data.wow.teams.players",
                                Query::terms("data.wow.teams.players.info.data.guid", guids.clone()),
                            )
                        )
                    );
                }

                if let Some(encounter_difficulties) = self.encounter_difficulties.as_ref() {
                    q = q.filter(Query::terms("data.wow.encounter.difficulty", encounter_difficulties.clone()));
                }

                {
                    let mut keystone_query = Query::range("data.wow.challenge.keystoneLevel");
                    if let Some(ks_low) = self.keystone_low {
                        keystone_query = keystone_query.gte(ks_low);
                    }

                    if let Some(ks_high) = self.keystone_low {
                        keystone_query = keystone_query.lte(ks_high);
                    }
                    q = q.filter(keystone_query);
                }

                if let Some(instance_types) = self.instance_types.as_ref() {
                    q = q.filter(Query::terms("data.wow.instance.instanceType", instance_types.iter().map(|x| *x as i32).collect::<Vec<i32>>()));
                }

                {
                    let friendly = self.build_friendly_es_composition_filter();
                    let enemy = self.build_enemy_es_composition_filter();

                    q = q.filter(
                        Query::bool()
                            .minimum_should_match("1")
                            .should(
                                Query::bool()
                                    .filter(
                                        Query::nested(
                                            "data.wow.teams",
                                            if friendly.1 {
                                                Query::bool()
                                                    .filter(Query::term("data.wow.teams.team.id", 0i32))
                                                    .filter(friendly.0.clone())
                                            } else {
                                                Query::bool()
                                            }
                                        )
                                    )
                                    .filter(
                                        Query::nested(
                                            "data.wow.teams",
                                            if enemy.1 {
                                                Query::bool()
                                                    .filter(Query::term("data.wow.teams.team.id", 1i32))
                                                    .filter(enemy.0.clone())
                                            } else {
                                                Query::bool()
                                            }
                                        )
                                    )
                            )
                            .should(
                                Query::bool()
                                    .filter(
                                        Query::nested(
                                            "data.wow.teams",
                                            if friendly.1 {
                                                Query::bool()
                                                    .filter(Query::term("data.wow.teams.team.id", 1i32))
                                                    .filter(friendly.0.clone())
                                            } else {
                                                Query::bool()
                                            }
                                        )
                                    )
                                    .filter(
                                        Query::nested(
                                            "data.wow.teams",
                                            if enemy.1 {
                                                Query::bool()
                                                    .filter(Query::term("data.wow.teams.team.id", 0i32))
                                                    .filter(enemy.0.clone())
                                            } else {
                                                Query::bool()
                                            }
                                        )
                                    )
                            )
                    );
                }

                q
            })
    }

    pub fn all_instance_ids(&self) -> Vec<i32> {
        let mut instance_ids: Vec<i32> = vec![];
        if let Some(raids) = self.raids.as_ref() {
            instance_ids.extend(raids);
        }

        if let Some(dungeons) = self.dungeons.as_ref() {
            instance_ids.extend(dungeons);
        }
        
        if let Some(arenas) = self.arenas.as_ref() {
            instance_ids.extend(arenas);
        }
        instance_ids
    }

    pub fn build_friendly_es_composition_filter(&self) -> (NestedQuery, bool) {
        WowListQuery::build_es_composition_filter(self.friendly_composition.as_ref())
    }

    pub fn build_enemy_es_composition_filter(&self) -> (NestedQuery, bool) {
        WowListQuery::build_es_composition_filter(self.enemy_composition.as_ref())
    }

    fn build_es_composition_filter(f: Option<&Vec<String>>) -> (NestedQuery, bool) {
        let mut q = Query::bool();
        let mut has_filter = false;
        if let Some(inner) = f {
            for x in inner {
                let json_arr: Vec<i32> = serde_json::from_str(x).unwrap_or(vec![]);
                if json_arr.is_empty() {
                    continue;
                }
                has_filter = true;

                
                q = q.should(Query::terms("data.wow.teams.players.info.data.specId", json_arr));
            }

            if has_filter {
                q = q.minimum_should_match("1");
            }
        }

        (
            Query::nested("data.wow.teams.players", q),
            has_filter,
        )
    }
}

impl api::ApiApplication {
    async fn list_wow_encounters_for_character(&self, character_guid: &str, user_id: i64, req_user_id: i64, start: i64, end: i64, filters: &WowListQuery, machine_id: &str) -> Result<Vec<WoWEncounter>, SquadOvError> {
        let filter = RecentMatchQuery{
            games: Some(vec![SquadOvGames::WorldOfWarcraft]),
            users: Some(vec![user_id]),
            squads: Some(self.get_user_squads(req_user_id).await?.into_iter().map(|x| { x.squad.id }).collect()),
            filters: RecentMatchGameQuery{
                wow: GenericWowQuery{
                    encounters: WowListQuery{
                        guids: Some(vec![character_guid.to_string()]),
                        ..filters.clone()
                    },
                    keystones: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    arenas: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    instances: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    ..GenericWowQuery::default()
                },
                ..RecentMatchGameQuery::default()
            },
            ..RecentMatchQuery::default()
        };
    
        let es_search = filter.to_es_search(req_user_id, Some(&machine_id), false)
            .from(start)
            .size(end)
            .sort(vec![
                Sort::new("vod.endTime")
                    .order(SortOrder::Desc)
            ]);
        let pairs: Vec<_> = self.es_api.search_documents::<ESVodDocument>(&self.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?
                .into_iter()
                .map(|x| {
                    MatchPlayerPair{
                        match_uuid: x.data.match_uuid.unwrap_or(Uuid::new_v4()),
                        player_uuid: x.owner.uuid,
                    }
                })
                .collect();

        Ok(wm::list_wow_encounter_for_uuids(&*self.heavy_pool, &pairs).await?)
    }

    async fn list_wow_challenges_for_character(&self, character_guid: &str, user_id: i64, req_user_id: i64, start: i64, end: i64, filters: &WowListQuery, machine_id: &str) -> Result<Vec<WoWChallenge>, SquadOvError> {
        let filter = RecentMatchQuery{
            games: Some(vec![SquadOvGames::WorldOfWarcraft]),
            users: Some(vec![user_id]),
            squads: Some(self.get_user_squads(req_user_id).await?.into_iter().map(|x| { x.squad.id }).collect()),
            filters: RecentMatchGameQuery{
                wow: GenericWowQuery{
                    keystones: WowListQuery{
                        guids: Some(vec![character_guid.to_string()]),
                        ..filters.clone()
                    },
                    encounters: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    arenas: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    instances: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    ..GenericWowQuery::default()
                },
                ..RecentMatchGameQuery::default()
            },
            ..RecentMatchQuery::default()
        };
    
        let es_search = filter.to_es_search(req_user_id, Some(&machine_id), false)
            .from(start)
            .size(end)
            .sort(vec![
                Sort::new("vod.endTime")
                    .order(SortOrder::Desc)
            ]);
        let pairs: Vec<_> = self.es_api.search_documents::<ESVodDocument>(&self.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?
                .into_iter()
                .map(|x| {
                    MatchPlayerPair{
                        match_uuid: x.data.match_uuid.unwrap_or(Uuid::new_v4()),
                        player_uuid: x.owner.uuid,
                    }
                })
                .collect();
        Ok(wm::list_wow_challenges_for_uuids(&*self.heavy_pool, &pairs).await?)
    }

    async fn list_wow_arenas_for_character(&self, character_guid: &str, user_id: i64, req_user_id: i64, start: i64, end: i64, filters: &WowListQuery, machine_id: &str) -> Result<Vec<WoWArena>, SquadOvError> {
        let filter = RecentMatchQuery{
            games: Some(vec![SquadOvGames::WorldOfWarcraft]),
            users: Some(vec![user_id]),
            squads: Some(self.get_user_squads(req_user_id).await?.into_iter().map(|x| { x.squad.id }).collect()),
            filters: RecentMatchGameQuery{
                wow: GenericWowQuery{
                    arenas: WowListQuery{
                        guids: Some(vec![character_guid.to_string()]),
                        ..filters.clone()
                    },
                    encounters: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    keystones: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    instances: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    ..GenericWowQuery::default()
                },
                ..RecentMatchGameQuery::default()
            },
            ..RecentMatchQuery::default()
        };
    
        let es_search = filter.to_es_search(req_user_id, Some(&machine_id), false)
            .from(start)
            .size(end)
            .sort(vec![
                Sort::new("vod.endTime")
                    .order(SortOrder::Desc)
            ]);

        let pairs: Vec<_> = self.es_api.search_documents::<ESVodDocument>(&self.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?
                .into_iter()
                .map(|x| {
                    MatchPlayerPair{
                        match_uuid: x.data.match_uuid.unwrap_or(Uuid::new_v4()),
                        player_uuid: x.owner.uuid,
                    }
                })
                .collect();
        Ok(wm::list_wow_arenas_for_uuids(&*self.heavy_pool, &pairs).await?)
    }

    async fn list_wow_instances_for_character(&self, character_guid: &str, user_id: i64, req_user_id: i64, start: i64, end: i64, filters: &WowListQuery, machine_id: &str) -> Result<Vec<WowInstance>, SquadOvError> {
        let filter = RecentMatchQuery{
            games: Some(vec![SquadOvGames::WorldOfWarcraft]),
            users: Some(vec![user_id]),
            squads: Some(self.get_user_squads(req_user_id).await?.into_iter().map(|x| { x.squad.id }).collect()),
            filters: RecentMatchGameQuery{
                wow: GenericWowQuery{
                    instances: WowListQuery{
                        guids: Some(vec![character_guid.to_string()]),
                        ..filters.clone()
                    },
                    encounters: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    keystones: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    arenas: WowListQuery{
                        enabled: false,
                        ..WowListQuery::default()
                    },
                    ..GenericWowQuery::default()
                },
                ..RecentMatchGameQuery::default()
            },
            ..RecentMatchQuery::default()
        };
    
        let es_search = filter.to_es_search(req_user_id, Some(&machine_id), false)
            .from(start)
            .size(end)
            .sort(vec![
                Sort::new("vod.endTime")
                    .order(SortOrder::Desc)
            ]);
        let pairs: Vec<_> = self.es_api.search_documents::<ESVodDocument>(&self.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?
                .into_iter()
                .map(|x| {
                    MatchPlayerPair{
                        match_uuid: x.data.match_uuid.unwrap_or(Uuid::new_v4()),
                        player_uuid: x.owner.uuid,
                    }
                })
                .collect();
        Ok(wm::list_wow_instances_for_uuids(&*self.heavy_pool, &pairs).await?)
    }

    pub async fn get_wow_match_view_owner(&self, view_uuid: &Uuid) -> Result<i64, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT user_id
                FROM squadov.wow_match_view
                WHERE id = $1
                ",
                view_uuid,
            )
                .fetch_one(&*self.pool)
                .await?
                .user_id
        )
    }

    pub async fn get_wow_match_view_for_user_match(&self, user_id: i64, match_uuid: &Uuid) -> Result<Option<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT id
                FROM squadov.wow_match_view
                WHERE user_id = $1
                    AND match_uuid = $2
                ",
                user_id,
                match_uuid
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.id
                })
        )
    }

    async fn create_generic_wow_match_view<T>(&self, tx: &mut Transaction<'_, Postgres>, data: &GenericMatchCreationRequest<T>, user_id: i64) -> Result<Uuid, SquadOvError> {
        Ok(
            sqlx::query!(
                r#"
                INSERT INTO squadov.wow_match_view (
                    id,
                    user_id,
                    start_tm,
                    combat_log_version,
                    advanced_log,
                    build_version,
                    session_id
                )
                VALUES (
                    gen_random_uuid(),
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6
                )
                RETURNING id
                "#,
                user_id,
                &data.timestamp,
                &data.cl.combat_log_version,
                data.cl.advanced_log,
                &data.cl.build_version,
                data.session,
            )
                .fetch_one(tx)
                .await?
                .id
        )
    }

    pub async fn create_wow_encounter_match_view(&self, tx: &mut Transaction<'_, Postgres>, data: &GenericMatchCreationRequest<WoWEncounterStart>, user_id: i64) -> Result<Uuid, SquadOvError> {
        let uuid = self.create_generic_wow_match_view(&mut *tx, &data, user_id).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.wow_encounter_view (
                view_id,
                encounter_id,
                encounter_name,
                difficulty,
                num_players,
                instance_id
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6
            )
            ",
            &uuid,
            data.data.encounter_id,
            &data.data.encounter_name,
            data.data.difficulty,
            data.data.num_players,
            data.data.instance_id,
        )
            .execute(&mut *tx)
            .await?;
        Ok(uuid)
    }

    pub async fn find_existing_wow_encounter_match(&self, view_uuid: &Uuid, tm: &DateTime<Utc>, key: &str) -> Result<Option<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT match_uuid
                FROM squadov.new_wow_encounters AS wc
                CROSS JOIN (
                    SELECT wmv.start_tm, wcv.encounter_id, wcv.difficulty, wcv.instance_id
                    FROM squadov.wow_match_view AS wmv
                    INNER JOIN squadov.wow_encounter_view AS wcv
                        ON wcv.view_id = wmv.id
                    WHERE wmv.id = $2
                ) AS wmv(start_tm, encounter_id, difficulty, instance_id)
                WHERE wc.tr && tstzrange(wmv.start_tm, $3, '[]')
                    AND wc.combatants_key = $1
                    AND wc.encounter_id = wmv.encounter_id
                    AND wc.difficulty = wmv.difficulty
                    AND wc.instance_id = wmv.instance_id
                ",
                key,
                view_uuid,
                tm,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.match_uuid
                })
        )
    }

    pub async fn finish_wow_encounter_match(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, key: &str) -> Result<(), SquadOvError> {
        // Insert into wow encounters table.
        sqlx::query!(
            "
            INSERT INTO squadov.new_wow_encounters (
                match_uuid,
                tr,
                combatants_key,
                encounter_id,
                difficulty,
                instance_id
            )
            SELECT
                $1,
                tstzrange(wmv.start_tm, $4, '[]'),
                $2,
                wev.encounter_id,
                wev.difficulty,
                wev.instance_id
            FROM squadov.wow_match_view AS wmv
            INNER JOIN squadov.wow_encounter_view AS wev
                ON wev.view_id = wmv.id
            WHERE wmv.id = $3
            ",
            match_uuid,
            key,
            view_uuid,
            tm,
        )
            .execute(&mut *tx)
            .await?;

        Ok(())
    }

    pub async fn finish_wow_encounter_view(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, game: &WoWEncounterEnd) -> Result<(), SquadOvError> {
        // Modify view to link to the new match and to update the end time as well.
        sqlx::query!(
            "
            UPDATE squadov.wow_match_view
            SET end_tm = $2,
                match_uuid = $3
            WHERE id = $1
            ",
            view_uuid,
            tm,
            match_uuid,
        )
            .execute(&mut *tx)
            .await?;

        // Modify game specific view with data parameters.
        sqlx::query!(
            "
            UPDATE squadov.wow_encounter_view
            SET success = $2
            WHERE view_id = $1
            ",
            view_uuid,
            game.success,
        )
            .execute(&mut *tx)
            .await?;
        
        Ok(())
    }

    pub async fn create_wow_challenge_match_view(&self, tx: &mut Transaction<'_, Postgres>, data: &GenericMatchCreationRequest<WoWChallengeStart>, user_id: i64) -> Result<Uuid, SquadOvError> {
        let uuid = self.create_generic_wow_match_view(&mut *tx, data, user_id).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.wow_challenge_view (
                view_id,
                challenge_name,
                instance_id,
                keystone_level
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            &uuid,
            &data.data.challenge_name,
            data.data.instance_id,
            data.data.keystone_level,
        )
            .execute(&mut *tx)
            .await?;
        Ok(uuid)
    }

    pub async fn update_wow_challenge_view_uuid(&self, tx: &mut Transaction<'_, Postgres>, old_uuid: &Uuid, new_uuid: &Uuid) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.wow_challenge_view
            SET view_id = $2
            WHERE view_id = $1
            ",
            old_uuid,
            new_uuid,
        )
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    pub async fn find_existing_wow_challenge_match(&self, view_uuid: &Uuid, tm: &DateTime<Utc>, key: &str) -> Result<Option<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT match_uuid
                FROM squadov.new_wow_challenges AS wc
                CROSS JOIN (
                    SELECT wmv.start_tm, wcv.instance_id, wcv.keystone_level
                    FROM squadov.wow_match_view AS wmv
                    INNER JOIN squadov.wow_challenge_view AS wcv
                        ON wcv.view_id = wmv.id
                    WHERE wmv.id = $2
                ) AS wmv(start_tm, instance_id, keystone_level)
                WHERE wc.tr && tstzrange(wmv.start_tm, $3, '[]')
                    AND wc.combatants_key = $1
                    AND wc.instance_id = wmv.instance_id
                    AND wc.keystone_level = wmv.keystone_level
                ",
                key,
                view_uuid,
                tm,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.match_uuid
                })
        )
    }

    pub async fn finish_wow_challenge_view(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, game: &WoWChallengeEnd) -> Result<(), SquadOvError> {
        // Modify view to link to the new match and to update the end time as well.
        sqlx::query!(
            "
            UPDATE squadov.wow_match_view
            SET end_tm = $2,
                match_uuid = $3
            WHERE id = $1
            ",
            view_uuid,
            tm,
            match_uuid,
        )
            .execute(&mut *tx)
            .await?;

        // Modify game specific view with data parameters.
        sqlx::query!(
            "
            UPDATE squadov.wow_challenge_view
            SET success = $2,
                time_ms = $3
            WHERE view_id = $1
            ",
            view_uuid,
            game.success,
            game.time_ms,
        )
            .execute(&mut *tx)
            .await?;
        
        Ok(())
    }

    pub async fn finish_wow_challenge_match(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, key: &str) -> Result<(), SquadOvError> {
        // Insert into wow encounters table.
        sqlx::query!(
            "
            INSERT INTO squadov.new_wow_challenges (
                match_uuid,
                tr,
                combatants_key,
                instance_id,
                keystone_level
            )
            SELECT
                $1,
                tstzrange(wmv.start_tm, $4, '[]'),
                $2,
                wcv.instance_id,
                wcv.keystone_level
            FROM squadov.wow_match_view AS wmv
            INNER JOIN squadov.wow_challenge_view AS wcv
                ON wcv.view_id = wmv.id
            WHERE wmv.id = $3
            ON CONFLICT (match_uuid) DO NOTHING
            ",
            match_uuid,
            key,
            view_uuid,
            tm,
        )
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    pub async fn create_wow_arena_match_view(&self, tx: &mut Transaction<'_, Postgres>, data: &GenericMatchCreationRequest<WoWArenaStart>, user_id: i64) -> Result<Uuid, SquadOvError> {
        let uuid = self.create_generic_wow_match_view(&mut *tx, data, user_id).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.wow_arena_view (
                view_id,
                instance_id,
                arena_type
            )
            VALUES (
                $1,
                $2,
                $3
            )
            ",
            &uuid,
            data.data.instance_id,
            &data.data.arena_type
        )
            .execute(&mut *tx)
            .await?;
        Ok(uuid)
    }

    pub async fn find_existing_wow_arena_match(&self, view_uuid: &Uuid, tm: &DateTime<Utc>, key: &str) -> Result<Option<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT match_uuid
                FROM squadov.new_wow_arenas AS wc
                CROSS JOIN (
                    SELECT wmv.start_tm, wcv.instance_id, wcv.arena_type
                    FROM squadov.wow_match_view AS wmv
                    INNER JOIN squadov.wow_arena_view AS wcv
                        ON wcv.view_id = wmv.id
                    WHERE wmv.id = $2
                ) AS wmv(start_tm, instance_id, arena_type)
                WHERE wc.tr && tstzrange(wmv.start_tm, $3, '[]')
                    AND wc.combatants_key = $1
                    AND wc.instance_id = wmv.instance_id
                    AND wc.arena_type = wmv.arena_type
                ",
                key,
                view_uuid,
                tm,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.match_uuid
                })
        )
    }

    pub async fn finish_wow_arena_match(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, key: &str) -> Result<(), SquadOvError> {
        // Insert into wow encounters table.
        sqlx::query!(
            "
            INSERT INTO squadov.new_wow_arenas (
                match_uuid,
                tr,
                combatants_key,
                instance_id,
                arena_type
            )
            SELECT
                $1,
                tstzrange(wmv.start_tm, $4, '[]'),
                $2,
                wav.instance_id,
                wav.arena_type
            FROM squadov.wow_match_view AS wmv
            INNER JOIN squadov.wow_arena_view AS wav
                ON wav.view_id = wmv.id
            WHERE wmv.id = $3
            ",
            match_uuid,
            key,
            view_uuid,
            tm,
        )
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    pub async fn finish_wow_arena_view(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, game: &WoWArenaEnd) -> Result<(), SquadOvError> {
        // Modify view to link to the new match and to update the end time as well.
        sqlx::query!(
            "
            UPDATE squadov.wow_match_view
            SET end_tm = $2,
                match_uuid = $3
            WHERE id = $1
            ",
            view_uuid,
            tm,
            match_uuid,
        )
            .execute(&mut *tx)
            .await?;

        // Modify game specific view with data parameters.
        sqlx::query!(
            "
            UPDATE squadov.wow_arena_view
            SET winning_team_id = $2,
                match_duration_seconds = $3,
                new_ratings = $4
            WHERE view_id = $1
            ",
            view_uuid,
            game.winning_team_id,
            game.match_duration_seconds,
            &game.new_ratings,
        )
            .execute(&mut *tx)
            .await?;
        
        Ok(())
    }

    pub async fn create_wow_instance_match_view(&self, tx: &mut Transaction<'_, Postgres>, data: &GenericMatchCreationRequest<WowInstanceData>, user_id: i64) -> Result<Uuid, SquadOvError> {
        let uuid = self.create_generic_wow_match_view(&mut *tx, data, user_id).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.wow_instance_view (
                view_id,
                instance_id,
                instance_name,
                instance_type
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ",
            &uuid,
            data.data.id as i64,
            &data.data.name,
            data.data.instance_type as i32,
        )
            .execute(&mut *tx)
            .await?;
        Ok(uuid)
    }

    pub async fn find_existing_wow_instance_match(&self, view_uuid: &Uuid, tm: &DateTime<Utc>, players: &[i32]) -> Result<Option<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT match_uuid
                FROM squadov.new_wow_instances AS wc
                CROSS JOIN (
                    SELECT wmv.start_tm, wcv.instance_id, wcv.instance_type
                    FROM squadov.wow_match_view AS wmv
                    INNER JOIN squadov.wow_instance_view AS wcv
                        ON wcv.view_id = wmv.id
                    WHERE wmv.id = $1
                ) AS wmv(start_tm, instance_id, instance_type)
                WHERE wc.tr && tstzrange(wmv.start_tm, $2, '[]')
                    AND wc.instance_id = wmv.instance_id
                    AND wc.instance_type = wmv.instance_type
                    AND wc.players && $3::INTEGER[]
                ",
                view_uuid,
                tm,
                players,
            )
                .fetch_optional(&*self.pool)
                .await?
                .map(|x| {
                    x.match_uuid
                })
        )
    }

    pub async fn update_wow_instance_match_players(&self, tx: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, players: &[i32]) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.new_wow_instances
            SET players = players | $2::INTEGER[]
            WHERE match_uuid = $1
            ",
            match_uuid,
            players,
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn finish_wow_instance_match(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>, players: &[i32], combatants: &[WoWCombatantInfo]) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.new_wow_instances (
                match_uuid,
                tr,
                instance_id,
                instance_type,
                players,
                players_raw
            )
            SELECT
                $1,
                tstzrange(wmv.start_tm, $4, '[]'),
                wav.instance_id,
                wav.instance_type,
                $2,
                $5
            FROM squadov.wow_match_view AS wmv
            INNER JOIN squadov.wow_instance_view AS wav
                ON wav.view_id = wmv.id
            WHERE wmv.id = $3
            ",
            match_uuid,
            players,
            view_uuid,
            tm,
            &combatants.iter().map(|x| {
                x.guid.clone()
            }).collect::<Vec<String>>(),
        )
            .execute(&mut *tx)
            .await?;
        Ok(())
    }

    pub async fn delete_wow_instance_view(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid) -> Result<(), SquadOvError> {
        // ONLY DELETE IT FROM wow_instance_view.
        // NEVER EVER DELETE FROM wow_match_view PLEASE AND THANK YOU.
        sqlx::query!(
            "
            DELETE FROM squadov.wow_instance_view
            WHERE view_id = $1
            ",
            view_uuid,
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn finish_wow_generic_match_view(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, tm: &DateTime<Utc>) -> Result<(), SquadOvError> {
        // Modify view to link to the new match and to update the end time as well.
        sqlx::query!(
            "
            UPDATE squadov.wow_match_view
            SET end_tm = $2,
                match_uuid = $3
            WHERE id = $1
            ",
            view_uuid,
            tm,
            match_uuid,
        )
            .execute(&mut *tx)
            .await?;

        Ok(())
    }

    
    pub async fn finish_wow_match_view(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid) -> Result<(), SquadOvError> {
        // Need to populate player_rating, player_spec, t0_specs, and t1_specs in wow_match_view so that
        // we can provide easy filtering on them.
        sqlx::query!(
            r#"
            UPDATE squadov.wow_match_view AS wmv
            SET player_rating = sub.player_rating,
                player_spec = sub.player_spec,
                player_team = sub.player_team,
                t0_specs = sub.t0_specs,
                t1_specs = sub.t1_specs
            FROM (
                SELECT
                    wmv.id,
                    wvc.spec_id AS "player_spec",
                    wvc.rating AS "player_rating",
                    wvc.team AS "player_team",
                    t0.s AS "t0_specs",
                    t1.s AS "t1_specs"
                FROM squadov.wow_match_view AS wmv
                LEFT JOIN LATERAL (
                    SELECT wcp.character_id
                    FROM squadov.wow_match_view_character_presence AS wcp
                    INNER JOIN squadov.wow_user_character_cache AS wucc
                        ON wucc.user_id = wmv.user_id
                            AND wucc.unit_guid = wcp.unit_guid
                    WHERE wcp.view_id = wmv.id
                ) AS wcp
                    ON TRUE
                LEFT JOIN squadov.wow_match_view_combatants AS wvc
                    ON wvc.character_id = wcp.character_id
                LEFT JOIN LATERAL (
                    SELECT ',' || STRING_AGG(val::VARCHAR, ',') || ',' AS vv
                    FROM (
                        SELECT MIN(wvc.spec_id)
                        FROM squadov.wow_match_view_character_presence AS wcp
                        INNER JOIN squadov.wow_match_view_combatants AS wvc
                            ON wvc.character_id = wcp.character_id
                        WHERE wcp.view_id = wmv.id
                            AND wvc.team = 0
                        GROUP BY wcp.view_id, wcp.unit_guid
                    ) sub(val)
                ) AS t0(s)
                    ON TRUE
                LEFT JOIN LATERAL (
                    SELECT ',' || STRING_AGG(val::VARCHAR, ',') || ',' AS vv
                    FROM (
                        SELECT MIN(wvc.spec_id)
                        FROM squadov.wow_match_view_character_presence AS wcp
                        INNER JOIN squadov.wow_match_view_combatants AS wvc
                            ON wvc.character_id = wcp.character_id
                        WHERE wcp.view_id = wmv.id
                            AND wvc.team = 1
                        GROUP BY wcp.view_id, wcp.unit_guid
                    ) sub(val)
                ) AS t1(s)
                    ON TRUE
                WHERE wmv.id = $1
            ) AS sub
            WHERE sub.id = wmv.id
            "#,
            view_uuid,
        )
            .execute(tx)
            .await?;

        Ok(())
    }

    pub async fn group_wow_encounter_using_session(&self, tx: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid) -> Result<(), SquadOvError> {
        // First, find the previous encounter match that has the same session, is the same encounter, and has the same people running it.        
        let some_previous_match_uuid: Option<Uuid> = sqlx::query!(
            "
            WITH current AS (
                SELECT *
                FROM squadov.wow_match_view AS wmv
                INNER JOIN squadov.new_wow_encounters AS nwe
                    ON nwe.match_uuid = wmv.match_uuid
                WHERE wmv.id = $1
            )
            SELECT nwe.match_uuid
            FROM current
            INNER JOIN squadov.wow_match_view AS wmv
                ON wmv.user_id = current.user_id
                    AND wmv.session_id = current.session_id
            INNER JOIN squadov.new_wow_encounters AS nwe
                ON nwe.match_uuid = wmv.match_uuid
                    AND nwe.combatants_key = current.combatants_key
                    AND nwe.encounter_id = current.encounter_id
                    AND nwe.difficulty = current.difficulty
                    AND nwe.instance_id = current.instance_id
            WHERE wmv.id != current.id
            ORDER BY wmv.end_tm DESC
            LIMIT 1
            ",
            view_uuid,
        )
            .fetch_optional(&mut *tx)
            .await?
            .map(|x| {
                x.match_uuid
            });


        let collection_uuid = if let Some(previous_match_uuid) = some_previous_match_uuid {
            // The previous match should already be in a collection because the other branch of the parent if statement
            // creates the collection if it's the first one in.
            matches::get_match_collection_for_match(&mut *tx, &previous_match_uuid).await?
        } else {
            // This is the first pull presumably. Create a new collection that future pulls will be added to.
            matches::create_new_match_collection(&mut *tx).await?        
        };

        matches::add_match_to_collection(&mut *tx, match_uuid, &collection_uuid).await?;
        Ok(())
    }

    async fn list_ordered_wow_encounter_match_pulls(&self, match_uuid: &Uuid, user_id: i64) -> Result<Vec<Uuid>, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT mmc2.match_uuid
                FROM squadov.match_to_match_collection AS mmc
                INNER JOIN squadov.match_to_match_collection AS mmc2
                    ON mmc2.collection_uuid = mmc.collection_uuid
                INNER JOIN squadov.wow_match_view AS wmv
                    ON wmv.match_uuid = mmc2.match_uuid
                WHERE mmc.match_uuid = $1
                    AND wmv.user_id = $2
                ORDER BY wmv.end_tm ASC
                ",
                match_uuid,
                user_id,
            )
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|x| {
                    x.match_uuid
                })
                .collect::<Vec<Uuid>>()
        )
    }
}

pub async fn create_wow_encounter_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWEncounterStart>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_encounter_match_view(&mut tx, &input_match, session.user.id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}

pub async fn create_wow_challenge_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWChallengeStart>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_challenge_match_view(&mut tx, &input_match, session.user.id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}

pub async fn create_wow_arena_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WoWArenaStart>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_arena_match_view(&mut tx, &input_match, session.user.id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}

pub async fn finish_wow_encounter_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GenericMatchFinishCreationRequest<WoWEncounterEnd>>, path: web::Path<super::WoWViewPath>) -> Result<HttpResponse, SquadOvError> {
    let combatants_key = generate_combatants_key(&data.combatants);
    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let mut created_uuid: bool = false;
        let match_uuid = match app.find_existing_wow_encounter_match(&path.view_uuid, &data.timestamp, &combatants_key).await? {
            Some(uuid) => uuid,
            None => {
                let new_match = app.create_new_match(&mut tx, SquadOvGames::WorldOfWarcraft).await?;
                match app.finish_wow_encounter_match(&mut tx, &path.view_uuid, &new_match.uuid, &data.timestamp, &combatants_key).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW encounter...retrying!");
                            continue;
                        },
                        _ => return Err(err)
                    }
                };

                created_uuid = true;
                new_match.uuid
            }
        };
        app.finish_wow_encounter_view(&mut tx, &path.view_uuid, &match_uuid, &data.timestamp, &data.data).await?;
        app.finish_wow_match_view(&mut tx, &path.view_uuid).await?;

        if created_uuid {
            // Only the person who creates the new match UUID should be allowed to create a match collection
            // for consecutive pulls and add this match to a collection. This way we won't have to deal with
            // the possibility of multiple match collections all existing that have the same set of matches in them.
            // Note that this also has to be after .finish_wow_encounter_view() because otherwise the match uuid
            // in the match view won't be set.
            app.group_wow_encounter_using_session(&mut tx, &path.view_uuid, &match_uuid).await?;
        }

        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(SquadOvError::InternalError(String::from("Too many errors in finishing WoW encounter...Retry limit reached.")))
}

pub async fn create_wow_instance_match_handler(app : web::Data<Arc<api::ApiApplication>>, input_match: web::Json<GenericMatchCreationRequest<WowInstanceData>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::BadRequest)?;
    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_instance_match_view(&mut tx, &input_match, session.user.id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}

pub async fn finish_wow_instance_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWViewPath>, data: web::Json<GenericMatchFinishCreationRequest<Option<()>>>) -> Result<HttpResponse, SquadOvError> {
    let player_hashes = generate_combatants_hashed_array(&data.combatants)?;

    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;

        // There's a couple of things that we need to take care of here.
        // 1) If an existing match exists note that we need to update the players array to ensure that the list of players
        //    we have is fully up to date for this match.
        //      a) In the case of another conflict on the update then we technically have two matches that exist
        //         in the database that have disjoint sets of players in the same match with a single player
        //         that overlaps between the two. I think this case is unlikely to ever happen so we can just
        //         ignore it.
        // 2) If an existing match doesn't exist then we just need to create it as per usual.
        let match_uuid = match app.find_existing_wow_instance_match(&path.view_uuid, &data.timestamp, &player_hashes).await? {
            Some(uuid) => {
                match app.update_wow_instance_match_players(&mut tx, &uuid, &player_hashes).await {
                    Ok(_) => (),
                    Err(err) => log::warn!("Failed to update Wow instance match players: {:?}", err),
                };

                uuid
            },
            None => {
                let new_match = app.create_new_match(&mut tx, SquadOvGames::WorldOfWarcraft).await?;
                match app.finish_wow_instance_match(&mut tx, &path.view_uuid, &new_match.uuid, &data.timestamp, &player_hashes, &data.combatants).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW instance...retrying!");
                            continue;
                        },
                        _ => return Err(err)
                    }
                };
                new_match.uuid
            }
        };
        app.finish_wow_generic_match_view(&mut tx, &path.view_uuid, &match_uuid, &data.timestamp).await?;
        app.finish_wow_match_view(&mut tx, &path.view_uuid).await?;

        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(SquadOvError::InternalError(String::from("Too many errors in finishing WoW instance...Retry limit reached.")))
}

pub async fn convert_wow_instance_to_keystone_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWViewPath>, input_match: web::Json<GenericMatchCreationRequest<WoWChallengeStart>>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::BadRequest)?;

    let mut tx = app.pool.begin().await?;
    let uuid = app.create_wow_challenge_match_view(&mut tx, &input_match, session.user.id).await?;

    // Pretty much the same thing as the normal thing where we create a keystone view but
    // in this case we want to update the view to have the proper UUID specified in the path.
    app.update_wow_challenge_view_uuid(&mut tx, &uuid, &path.view_uuid).await?;

    // Also need to delete the old instance view since that's no longer necessary.
    app.delete_wow_instance_view(&mut tx, &path.view_uuid).await?;

    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}

pub async fn finish_wow_challenge_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GenericMatchFinishCreationRequest<WoWChallengeEnd>>, path: web::Path<super::WoWViewPath>) -> Result<HttpResponse, SquadOvError> {
    let combatants_key = generate_combatants_key(&data.combatants);
    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match app.find_existing_wow_challenge_match(&path.view_uuid, &data.timestamp, &combatants_key).await? {
            Some(uuid) => uuid,
            None => {
                let new_match = app.create_new_match(&mut tx, SquadOvGames::WorldOfWarcraft).await?;
                match app.finish_wow_challenge_match(&mut tx, &path.view_uuid, &new_match.uuid, &data.timestamp, &combatants_key).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW challenge...retrying!");
                            continue;
                        },
                        _ => return Err(err)
                    }
                };
                new_match.uuid
            }
        };
        app.finish_wow_challenge_view(&mut tx, &path.view_uuid, &match_uuid, &data.timestamp, &data.data).await?;
        app.finish_wow_match_view(&mut tx, &path.view_uuid).await?;

        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(SquadOvError::InternalError(String::from("Too many errors in finishing WoW challenge...Retry limit reached.")))
}

pub async fn finish_wow_arena_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GenericMatchFinishCreationRequest<WoWArenaEnd>>, path: web::Path<super::WoWViewPath>) -> Result<HttpResponse, SquadOvError> {
    let combatants_key = generate_combatants_key(&data.combatants);
    for _i in 0i32..2 {
        let mut tx = app.pool.begin().await?;
        let match_uuid = match app.find_existing_wow_arena_match(&path.view_uuid, &data.timestamp, &combatants_key).await? {
            Some(uuid) => uuid,
            None => {
                let new_match = app.create_new_match(&mut tx, SquadOvGames::WorldOfWarcraft).await?;
                match app.finish_wow_arena_match(&mut tx, &path.view_uuid, &new_match.uuid, &data.timestamp, &combatants_key).await {
                    Ok(_) => (),
                    Err(err) => match err {
                        SquadOvError::Duplicate => {
                            // This indicates that the match UUID is INVALID because a match with the same
                            // match ID already exists. Retry!
                            log::warn!("Caught duplicate WoW arena...retrying!");
                            continue;
                        },
                        _ => return Err(err)
                    }
                };
                new_match.uuid
            }
        };
        app.finish_wow_arena_view(&mut tx, &path.view_uuid, &match_uuid, &data.timestamp, &data.data).await?;
        app.finish_wow_match_view(&mut tx, &path.view_uuid).await?;

        tx.commit().await?;
        return Ok(HttpResponse::Ok().json(match_uuid));
    }
    Err(SquadOvError::InternalError(String::from("Too many errors in finishing WoW arena...Retry limit reached.")))
}

pub async fn list_wow_encounters_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, filters: web::Json<WowListQuery>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    let query = query.into_inner();
    let encounters = app.list_wow_encounters_for_character(
        &path.character_guid,
        path.user_id,
        session.user.id,
        query.start,
        query.end,
        &filters,
        &machine_id.id,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = encounters.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(encounters, &req, &query, expected_total == got_total)?))
}

pub async fn list_wow_challenges_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, filters: web::Json<WowListQuery>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;
    
    let query = query.into_inner();
    let challenges = app.list_wow_challenges_for_character(
        &path.character_guid,
        path.user_id,
        session.user.id,
        query.start,
        query.end,
        &filters,
        &machine_id.id,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = challenges.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(challenges, &req, &query, expected_total == got_total)?))
}

pub async fn list_wow_arenas_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, filters: web::Json<WowListQuery>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    let query = query.into_inner();
    let challenges = app.list_wow_arenas_for_character(
        &path.character_guid,
        path.user_id,
        session.user.id,
        query.start,
        query.end,
        &filters,
        &machine_id.id,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = challenges.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(challenges, &req, &query, expected_total == got_total)?))
}

pub async fn list_wow_instances_for_character_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<api::PaginationParameters>, filters: web::Json<WowListQuery>, path: web::Path<super::WoWUserCharacterPath>, req: HttpRequest, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    let query = query.into_inner();
    let instances = app.list_wow_instances_for_character(
        &path.character_guid,
        path.user_id,
        session.user.id,
        query.start,
        query.end,
        &filters,
        &machine_id.id,
    ).await?;

    let expected_total = query.end - query.start;
    let got_total = instances.len() as i64;
    Ok(HttpResponse::Ok().json(api::construct_hal_pagination_response(instances, &req, &query, expected_total == got_total)?))
}

pub async fn get_wow_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = extensions.get::<SquadOVSession>().ok_or(SquadOvError::Unauthorized)?;

    #[derive(Serialize)]
    struct Response {
        encounter: Option<WoWEncounter>,
        challenge: Option<WoWChallenge>,
        arena: Option<WoWArena>,
        instance: Option<WowInstance>,
    }

    let filter = RecentMatchQuery{
        games: Some(vec![SquadOvGames::WorldOfWarcraft]),
        matches: Some(vec![path.match_uuid.clone()]),
        users: Some(vec![path.user_id]),
        squads: Some(app.get_user_squads(session.user.id).await?.into_iter().map(|x| { x.squad.id }).collect()),
        ..RecentMatchQuery::default()
    };

    let es_search = filter.to_es_search(session.user.id, None, false);
    if let Some(document) = app.es_api.search_documents::<ESVodDocument>(&app.config.elasticsearch.vod_index_read, serde_json::to_value(es_search)?).await?.pop() {
        if let Some(wow) = document.data.wow {
            Ok(HttpResponse::Ok().json(Response{
                encounter: wow.encounter,
                challenge: wow.challenge,
                arena: wow.arena,
                instance: wow.instance,
            }))
        } else {
            Err(SquadOvError::NotFound)
        }
    } else {
        Err(SquadOvError::NotFound)
    }
}

#[derive(Serialize)]
struct WowUserAccessibleVodOutput {
    pub vods: Vec<VodAssociation>,
    #[serde(rename="userToId")]
    pub user_to_id: HashMap<Uuid, i64>
}

pub async fn list_wow_vods_for_squad_in_match_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<GenericMatchPathInput>, req: HttpRequest, machine_id: web::Header<SquadOvMachineId>) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(s) => s,
        None => return Err(SquadOvError::Unauthorized),
    };
    let vods = vdb::find_accessible_vods_in_match_for_user(&*app.pool, &path.match_uuid, session.user.id, &machine_id.id).await?;

    // Note that for each VOD we also need to figure out the mapping from user uuid to puuid.
    let user_uuids: Vec<Uuid> = vods.iter()
        .filter(|x| { x.user_uuid.is_some() })
        .map(|x| { x.user_uuid.unwrap().clone() })
        .collect();

    let user_uuid_to_id = app.get_user_uuid_to_user_id_map(&user_uuids).await?;

    Ok(HttpResponse::Ok().json(WowUserAccessibleVodOutput{
        vods,
        user_to_id: user_uuid_to_id,
    }))
}

pub async fn list_wow_match_pulls_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::WoWUserMatchPath>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        app.list_ordered_wow_encounter_match_pulls(&path.match_uuid, path.user_id).await?
    ))
}

pub async fn link_wow_match_view_to_combat_log_handler(app : web::Data<Arc<api::ApiApplication>>, view: web::Path<super::WoWViewPath>, cl: web::Path<super::WowCombatLogPath>) -> Result<HttpResponse, SquadOvError> {
    sqlx::query!(
        "
        UPDATE squadov.wow_match_view
        SET combat_log_partition_id = $2
        WHERE id = $1
        ",
        &view.view_uuid,
        &cl.partition_id
    )
        .execute(&*app.pool)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}