use async_std::sync::{RwLock, Arc};
use squadov_common::{
    RawWoWCombatLogPayload,
    SquadOvError,
    WoWCombatLogEvent,
    GenericWoWMatchView,
};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{Consumer};
use rdkafka::config::ClientConfig;
use rdkafka::Message;
use uuid::Uuid;
use crate::api;
use lru::LruCache;

const WOW_COMBAT_LOG_BUFFER_CAPACITY: usize = 1000;
const WOW_VIEW_LRU_CACHE_SIZE: usize = 32;

#[derive(Clone)]
struct WoWKafkaOpaque {
    app: Arc<api::ApiApplication>,
    events: Arc<RwLock<Vec<WoWCombatLogEvent>>>,
    view_cache: Arc<RwLock<LruCache<Uuid, GenericWoWMatchView>>>
}

pub fn create_wow_consumer_thread(app: Arc<api::ApiApplication>, topic: &str, cfg: &ClientConfig) -> tokio::task::JoinHandle<()> {
    let mut cfg = cfg.clone();
    cfg.set("group.id", "squadov_primary_wow_combat_logs_cg");

    let wow_consumer: StreamConsumer = cfg.create().unwrap();
    wow_consumer.subscribe(&vec![topic]).unwrap();
    tokio::task::spawn(async move {
        let wow_opaque = WoWKafkaOpaque{
            app,
            events: Arc::new(RwLock::new(Vec::with_capacity(WOW_COMBAT_LOG_BUFFER_CAPACITY))),
            view_cache: Arc::new(RwLock::new(LruCache::new(WOW_VIEW_LRU_CACHE_SIZE))),
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
            let match_view_uuid = Uuid::parse_str(&key)?;
            
            let match_view = {
                if let Some(view) = {
                    let mut view_cache = opaque.view_cache.write().await;
                    view_cache.get(&match_view_uuid).cloned()
                } {
                    view
                } else {
                    let view = squadov_common::get_generic_wow_match_view_from_id(&*opaque.app.pool, &match_view_uuid).await?;

                    let mut view_cache = opaque.view_cache.write().await;
                    view_cache.put(match_view_uuid.clone(), view.clone());

                    view
                }
            };

            // Note that the client does a simple split on the commas which is insufficient for advanced parsing
            // so we need to reparse so that at least the upper level parts corresponds to each logical section of
            // the log (i.e. for COMBATANT_INFO, the 29th element should be a string representing an array of all the user's items).
            let mut parsed_payload: RawWoWCombatLogPayload = serde_json::from_str(payload)?;
            if parsed_payload.version < 2 {
                log::warn!("Skipping WoW combat log due to old version.");
                return Err(SquadOvError::BadRequest);
            }

            parsed_payload.redo_parts();

            let mut manual_handle_flags = false;
            let is_view_complete;
            if parsed_payload.is_finish_token() {
                log::info!("Detect Finish Token for WoW Match View: {}", &match_view_uuid);
                is_view_complete = true;
            } else {
                let parsed_event = squadov_common::parse_raw_wow_combat_log_payload(&match_view_uuid, match_view.alt_id, match_view.user_id, &match_view.combat_log_state(), &parsed_payload)?;

                let mut events = opaque.events.write().await;
                for new_event in parsed_event {
                    // We want to flush logs on ENCOUNTER_END/COMBAT_CHALLENGE_END/ARENA_MATCH_END so that the entire
                    // match is available as soon as it's finished and not have to rely on more events
                    // being pushed onto the Kafka queue or waiting for the user to end the game.
                    manual_handle_flags = manual_handle_flags || match &new_event.event {
                        squadov_common::WoWCombatLogEventType::ChallengeModeEnd{..} | squadov_common::WoWCombatLogEventType::EncounterEnd{..} | squadov_common::WoWCombatLogEventType::ArenaEnd{..} => true,
                        _ => false
                    };

                    events.push(new_event);
                }

                is_view_complete = manual_handle_flags;
            }

            let handle_events = manual_handle_flags || parsed_payload.is_finish_token() ||{
                let events = opaque.events.read().await;
                events.len() >= WOW_COMBAT_LOG_BUFFER_CAPACITY
            };

            if handle_events {
                let mut tx = opaque.app.heavy_pool.begin().await?;
                let events = {
                    let mut events = opaque.events.write().await;
                    let ret = events.clone();
                    events.clear();
                    ret
                };
                log::info!("Handling {} WoW Combat Log Events", events.len());
                squadov_common::store_wow_combat_log_events(&mut tx, events).await?;
                tx.commit().await?;
            }

            if is_view_complete {
                // Need to get the match view again since we probably cached it from when no match uuid exists.
                let match_view = squadov_common::get_generic_wow_match_view_from_id(&*opaque.app.pool, &match_view_uuid).await?;
                if let Some(match_uuid) = match_view.match_uuid {
                    log::info!("...Sending Sync Match VOD: {} {}", &match_uuid, match_view.user_id);
                    opaque.app.es_itf.request_sync_match(match_uuid.clone(), Some(match_view.user_id)).await?;
                }
            }

            Ok(handle_events)
        }).await;
    })
}
