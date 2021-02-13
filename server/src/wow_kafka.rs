use async_std::sync::{RwLock, Arc};
use squadov_common::{RawWoWCombatLogPayload, SquadOvError, WoWCombatLogEvent};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{Consumer};
use rdkafka::config::ClientConfig;
use rdkafka::Message;
use uuid::Uuid;
use crate::api;

const WOW_COMBAT_LOG_BUFFER_CAPACITY: usize = 1000;
#[derive(Clone)]
struct WoWKafkaOpaque {
    app: Arc<api::ApiApplication>,
    events: Arc<RwLock<Vec<WoWCombatLogEvent>>>,
}

pub fn create_wow_consumer_thread(app: Arc<api::ApiApplication>, cfg: &ClientConfig) -> tokio::task::JoinHandle<()> {
    let mut cfg = cfg.clone();
    cfg.set("group.id", "squadov_primary_wow_combat_logs_cg");

    let wow_consumer: StreamConsumer = cfg.create().unwrap();
    wow_consumer.subscribe(&vec!["wow_combat_logs"]).unwrap();
    tokio::task::spawn(async move {
        let wow_opaque = WoWKafkaOpaque{
            app,
            events: Arc::new(RwLock::new(Vec::with_capacity(WOW_COMBAT_LOG_BUFFER_CAPACITY))),
        };

        squadov_common::generic_kafka_message_loop(wow_consumer, wow_opaque, |m, opaque| async move {
            let payload = match m.payload_view::<str>() {
                None => return Err(SquadOvError::NotFound),
                Some(Ok(s)) => s,
                Some(Err(e)) => return Err(SquadOvError::InternalError(format!("Kafka Payload View: {}", e)))
            };

            let key = m.key();
            if key.is_none() {
                log::warn!("Skipping WoW combat log message due to no key.");
                return Err(SquadOvError::BadRequest);
            }
            let key = std::str::from_utf8(key.unwrap())?;
            let combat_log_uuid = Uuid::parse_str(&key)?;
            let combat_log = opaque.app.get_wow_combat_log(&combat_log_uuid).await?;

            // Note that the client does a simple split on the commas which is insufficient for advanced parsing
            // so we need to reparse so that at least the upper level parts corresponds to each logical section of
            // the log (i.e. for COMBATANT_INFO, the 29th element should be a string representing an array of all the user's items).
            let mut parsed_payload: RawWoWCombatLogPayload = serde_json::from_str(payload)?;
            parsed_payload.redo_parts();

            let mut manual_handle_flags = false;
            if parsed_payload.is_finish_token() {
                log::info!("Detect Finish Token for WoW Combat Log: {}", &combat_log_uuid);
            } else {
                let parsed_event = squadov_common::parse_raw_wow_combat_log_payload(&combat_log_uuid, &combat_log.state, &parsed_payload)?;
                if parsed_event.is_some() {
                    let mut events = opaque.events.write().await;
                    let new_event = parsed_event.unwrap();

                    // We want to flush logs on ENCOUNTER_END/COMBAT_CHALLENGE_END/ARENA_MATCH_END so that the entire
                    // match is available as soon as it's finished and not have to rely on more events
                    // being pushed onto the Kafka queue or waiting for the user to end the game.
                    manual_handle_flags = match &new_event.event {
                        squadov_common::WoWCombatLogEventType::ChallengeModeEnd{..} | squadov_common::WoWCombatLogEventType::EncounterEnd{..} | squadov_common::WoWCombatLogEventType::ArenaEnd{..} => true,
                        _ => false
                    };

                    events.push(new_event);
                }
            }

            let handle_events = manual_handle_flags || parsed_payload.is_finish_token() ||{
                let events = opaque.events.read().await;
                events.len() >= WOW_COMBAT_LOG_BUFFER_CAPACITY
            };

            if handle_events {
                let mut tx = opaque.app.pool.begin().await?;
                let events = {
                    let events = opaque.events.read().await;
                    events.clone()
                };
                log::info!("Handling {} WoW Combat Log Events", events.len());
                let (unit_ownership, char_ownership, char_presence) = squadov_common::store_wow_combat_log_events(&mut tx, &events).await?;
                squadov_common::store_combat_log_unit_ownership_mapping(&mut tx, &unit_ownership).await?;
                squadov_common::store_combat_log_user_character_mapping(&mut tx, &char_ownership).await?;
                squadov_common::store_combat_log_character_presence(&mut tx, &char_presence).await?;
                tx.commit().await?;

                let mut events = opaque.events.write().await;
                events.clear();
            }

            Ok(handle_events)
        }).await;
    })
}
