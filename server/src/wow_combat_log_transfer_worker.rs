#[macro_use]
extern crate log;

mod api;

use squadov_common::{
    SquadOvError,
    rabbitmq::{
        RabbitMqInterface,
        RabbitMqListener,
    },
    wow::{
        matches,
        characters,
        reports::{
            characters::{
                WowCombatantReport,
                COMBATANT_REPORT_SCHEMA,
                WowCharacterReport,
                CHAR_REPORT_SCHEMA,
            },
            WowReportTypes,
            events::{
                aura_breaks::{
                    WowAuraBreakEventReport,
                    REPORT_SCHEMA as AURA_BREAK_REPORT_SCHEMA,
                },
                auras::{
                    WowAuraEventReport,
                    REPORT_SCHEMA as AURAS_REPORT_SCHEMA,
                },
                deaths::{
                    WowDeathEventReport,
                    WowDeathRecapHpEvent,
                    REPORT_SCHEMA as DEATHS_REPORT_SCHEMA,
                    DEATH_RECAP_SCHEMA,
                },
                encounters::{
                    WowEncounterEventReport,
                    REPORT_SCHEMA as ENCOUNTERS_REPORT_SCHEMA,
                },
                resurrections::{
                    WowResurrectionEventReport,
                    REPORT_SCHEMA as RESURRECTIONS_REPORT_SCHEMA,
                },
                spell_casts::{
                    WowSpellCastEventReport,
                    REPORT_SCHEMA as SPELL_CAST_REPORT_SCHEMA,
                },
            },
            stats::{
                WowUnitTimelineEntry,
                TIMELINE_SCHEMA,
                WowUnitStatSummary,
                SUMMARY_SCHEMA,
            },
        }
    }
};
use structopt::StructOpt;
use std::{fs, sync::Arc, collections::HashMap};
use uuid::Uuid;
use async_trait::async_trait;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
    #[structopt(short, long)]
    workers: usize,
    #[structopt(short, long)]
    threads: usize,
    #[structopt(short, long)]
    manual: Option<Uuid>,
}

pub struct WowTaskHandler {
    app: Arc<api::ApiApplication>,
}

impl WowTaskHandler {
    pub fn new (app: Arc<api::ApiApplication>) -> Self {
        Self {
            app,
        }
    }
}

impl WowTaskHandler {
    async fn handle_transfer_reports(&self, view_id: &Uuid) -> Result<(), SquadOvError>{
        // We need to do the same thing as the combat log report generator except that instead of creating them from
        // the parsed combat log, we need to create them from the existing data.
        let match_view = matches::get_generic_wow_match_view_from_id(&*self.app.pool, view_id).await?;
        let partition_id = format!("wow_{}", &match_view.id);

        if let Some(match_uuid) = match_view.match_uuid.as_ref() {
            // Character Reports
            // - Combatants
            // - Characters
            // - Loadouts
            {
                let combatants = characters::list_wow_characters_for_match(&*self.app.pool, match_uuid, match_view.user_id).await?;
                let combatant_guids: Vec<_> = combatants.iter().map(|x| { x.guid.clone() }).collect();

                for c in &combatants {
                    let loadout = characters::get_wow_full_character(&*self.app.pool, &match_view.id, &c.guid).await?;
                    self.app.cl_itf.save_report_json(&partition_id, WowReportTypes::CharacterLoadout as i32, &format!("{}.json", &c.guid), loadout).await?
                }

                // Stat Reports
                // - DPS
                // - HPS
                // - DRPS
                // - Summary
                {
                    let mut combatant_summaries: HashMap<String, WowUnitStatSummary> = HashMap::new();
                    for s in self.app.get_wow_summary_damage_dealt(match_view.user_id, match_uuid, &combatant_guids).await? {
                        if let Some(summary) = combatant_summaries.get_mut(&s.guid) {
                            summary.damage_dealt = s.value;
                        } else {
                            combatant_summaries.insert(s.guid.clone(), WowUnitStatSummary{
                                guid: s.guid.clone(),
                                damage_dealt: s.value,
                                damage_received: 0,
                                heals: 0,
                            });
                        }
                    }

                    for s in self.app.get_wow_summary_damage_received(match_view.user_id, match_uuid, &combatant_guids).await? {
                        if let Some(summary) = combatant_summaries.get_mut(&s.guid) {
                            summary.damage_received = s.value;
                        } else {
                            combatant_summaries.insert(s.guid.clone(), WowUnitStatSummary{
                                guid: s.guid.clone(),
                                damage_dealt: 0,
                                damage_received: s.value,
                                heals: 0,
                            });
                        }
                    }

                    for s in self.app.get_wow_summary_heals(match_view.user_id, match_uuid, &combatant_guids).await? {
                        if let Some(summary) = combatant_summaries.get_mut(&s.guid) {
                            summary.heals = s.value;
                        } else {
                            combatant_summaries.insert(s.guid.clone(), WowUnitStatSummary{
                                guid: s.guid.clone(),
                                damage_dealt: 0,
                                damage_received: 0,
                                heals: s.value,
                            });
                        }
                    }

                    self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Stats as i32, "summary.avro", &SUMMARY_SCHEMA, combatant_summaries.into_values().collect::<Vec<_>>()).await?;
                }

                {
                    let dps: Vec<_> = self.app.get_wow_match_dps(match_view.user_id, match_uuid, &combatant_guids, &api::v1::WowStatsQueryParams{
                        ps_step_seconds: 5,
                        start: None,
                        end: None,
                    }).await?
                        .into_iter()
                        .map(|(k,v)| {
                            v.into_iter().map(move |x| {
                                WowUnitTimelineEntry{
                                    guid: k.clone(),
                                    tm: x.tm as i64,
                                    value: x.value,
                                }
                            })
                        })
                        .flatten()
                        .collect();
                    self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Stats as i32, "dps.avro", &TIMELINE_SCHEMA, dps).await?;

                    let hps = self.app.get_wow_match_heals_per_second(match_view.user_id, match_uuid, &combatant_guids, &api::v1::WowStatsQueryParams{
                        ps_step_seconds: 5,
                        start: None,
                        end: None,
                    }).await?
                        .into_iter()
                        .map(|(k,v)| {
                            v.into_iter().map(move |x| {
                                WowUnitTimelineEntry{
                                    guid: k.clone(),
                                    tm: x.tm as i64,
                                    value: x.value,
                                }
                            })
                        })
                        .flatten()
                        .collect();
                    self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Stats as i32, "hps.avro", &TIMELINE_SCHEMA, hps).await?;

                    let drps = self.app.get_wow_match_damage_received_per_second(match_view.user_id, match_uuid, &combatant_guids, &api::v1::WowStatsQueryParams{
                        ps_step_seconds: 5,
                        start: None,
                        end: None,
                    }).await?
                        .into_iter()
                        .map(|(k,v)| {
                            v.into_iter().map(move |x| {
                                WowUnitTimelineEntry{
                                    guid: k.clone(),
                                    tm: x.tm as i64,
                                    value: x.value,
                                }
                            })
                        })
                        .flatten()
                        .collect();
                    self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Stats as i32, "drps.avro", &TIMELINE_SCHEMA, drps).await?;
                }

                self.app.cl_itf.save_report_avro(
                    &partition_id,
                    WowReportTypes::MatchCombatants as i32,
                    "combatants.avro",
                    &COMBATANT_REPORT_SCHEMA,
                    combatants
                        .into_iter()
                        .map(|x| { x.into() })
                        .collect::<Vec<WowCombatantReport>>(),
                ).await?;
            }

            {
                // This is never needed after the fact currently aside from ES document generation so we can ignore it in the transfer.
                let characters: Vec<WowCharacterReport> = vec![];
                self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::MatchCharacters as i32, "characters.avro", &CHAR_REPORT_SCHEMA, characters).await?;
            }
        }

        // Event Reports
        // - Deaths
        // - Auras
        // - Encounters
        // - Resurrections
        // - Aura Breaks
        // - Spell Casts
        // - Death Recaps
        {
            let deaths: Vec<WowDeathEventReport> = self.app.get_wow_match_death_events(&match_view.id).await?.into_iter().map(|x| { x.into() }).collect();
            for d in &deaths {
                let recap_events: Vec<WowDeathRecapHpEvent> = self.app.get_wow_death_recap(&match_view.id, d.event_id, 5).await?.hp_events.into_iter().map(|x| { x.into() }).collect();
                self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::DeathRecap as i32, &format!("{}.avro", d.event_id), &DEATH_RECAP_SCHEMA, recap_events).await?;
            }
            self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Events as i32, "deaths.avro", &DEATHS_REPORT_SCHEMA, deaths).await?;

            let auras: Vec<WowAuraEventReport> = self.app.get_wow_match_aura_events(&match_view.id).await?.into_iter().map(|x| { x.into() }).collect();
            self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Events as i32, "auras.avro", &AURAS_REPORT_SCHEMA, auras).await?;

            let encounters: Vec<WowEncounterEventReport> = self.app.get_wow_match_subencounters(&match_view.id).await?.into_iter().map(|x| { x.into() }).collect();
            self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Events as i32, "encounters.avro", &ENCOUNTERS_REPORT_SCHEMA, encounters).await?;

            let resurrections: Vec<WowResurrectionEventReport> = self.app.get_wow_match_resurrection_events(&match_view.id).await?.into_iter().map(|x| { x.into() }).collect();
            self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Events as i32, "resurrections.avro", &RESURRECTIONS_REPORT_SCHEMA, resurrections).await?;

            let aura_breaks: Vec<WowAuraBreakEventReport> = self.app.get_wow_match_aura_break_events(&match_view.id).await?.into_iter().map(|x| { x.into() }).collect();
            self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Events as i32, "aura_breaks.avro", &AURA_BREAK_REPORT_SCHEMA, aura_breaks).await?;

            let spell_casts: Vec<WowSpellCastEventReport> = self.app.get_wow_match_spell_cast_events(&match_view.id).await?.into_iter().map(|x| { x.into() }).collect();
            self.app.cl_itf.save_report_avro(&partition_id, WowReportTypes::Events as i32, "spell_casts.avro", &SPELL_CAST_REPORT_SCHEMA, spell_casts).await?;
        }
    
        Ok(())
    }
}

#[async_trait]
impl RabbitMqListener for WowTaskHandler {
    async fn handle(&self, data: &[u8], _queue: &str) -> Result<(), SquadOvError> {
        let view_ids: Vec<Uuid> = serde_json::from_slice(data)?;
        log::info!("Handle Combat Log Transfer RabbitMQ Task: {:?}", &view_ids);
        for view_id in view_ids {
            if let Err(err) = self.handle_transfer_reports(&view_id).await {
                log::error!("Failed to transfer report: {:?}", err);
            }
        }

        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,wow_combat_log_transfer_worker=debug,actix_web=debug,actix_http=debug,sqlx=info");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config.clone()).unwrap();
    let mut config : api::ApiConfig = toml::from_str(&raw_cfg).unwrap();
    config.rabbitmq.additional_queues = Some(vec!["wow_combat_log_transfer".to_string()]);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(opts.workers)
        .build()
        .unwrap()
        .block_on(async move {
            // Only use the provided config to connect to things.
            tokio::task::spawn(async move {
                let app = Arc::new(api::ApiApplication::new(&config, "wow_combat_log_transfer_worker").await);
                let handler_itf = Arc::new(WowTaskHandler::new(app.clone()));

                if let Some(manual) = opts.manual {
                    handler_itf.handle_transfer_reports(&manual).await.unwrap();
                } else {
                    for _i in 0..opts.threads {
                        RabbitMqInterface::add_listener(app.rabbitmq.clone(), "wow_combat_log_transfer".to_string(), handler_itf.clone(), 1).await.unwrap();
                    }
    
                    loop {
                        async_std::task::sleep(std::time::Duration::from_secs(1)).await;
                    }
                }
            }).await.unwrap();
            Ok(())
        })
}