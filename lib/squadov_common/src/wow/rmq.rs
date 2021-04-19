use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use async_std::sync::{RwLock, Arc};
use crate::{
    SquadOvError,
    rabbitmq::{RabbitMqInterface, RabbitMqListener, RabbitMqConfig, RABBITMQ_DEFAULT_PRIORITY},
    RawWoWCombatLogPayload,
    WoWCombatLogEvent,
    GenericWoWMatchView,
};
use sqlx::postgres::{PgPool};
use uuid::Uuid;
use lru::LruCache;
use std::io::Read;

const WOW_VIEW_LRU_CACHE_SIZE: usize = 32;

pub struct WowCombatLogRmqInterface {
    mqconfig: RabbitMqConfig,
    rmq: Arc<RabbitMqInterface>,
    db: Arc<PgPool>,
    view_cache: Arc<RwLock<LruCache<Uuid, GenericWoWMatchView>>>,
}

impl WowCombatLogRmqInterface {
    pub fn new (mqconfig: &RabbitMqConfig, rmq: Arc<RabbitMqInterface>, db: Arc<PgPool>) -> Self {
        Self {
            mqconfig: mqconfig.clone(),
            rmq,
            db,
            view_cache: Arc::new(RwLock::new(LruCache::new(WOW_VIEW_LRU_CACHE_SIZE)))
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WowCombatLogTask {
    Bulk{
        view_uuid: Uuid,
        raw_data: Vec<u8>,
    }
}

#[async_trait]
impl RabbitMqListener for WowCombatLogRmqInterface {
    async fn handle(&self, data: &[u8]) -> Result<(), SquadOvError> {
        let task: WowCombatLogTask = serde_json::from_slice(data)?;
        match task {
            WowCombatLogTask::Bulk{view_uuid, raw_data} => self.handle_multiple_combat_log_payload(&view_uuid, raw_data).await?,
        };
        Ok(())
    }
}

impl WowCombatLogRmqInterface {
    pub async fn handle_multiple_combat_log_payload(&self, view_uuid: &Uuid, raw_data: Vec<u8>) -> Result<(), SquadOvError> {
        let match_view = {
            if let Some(view) = {
                let mut view_cache = self.view_cache.write().await;
                view_cache.get(view_uuid).cloned()
            } {
                view
            } else {
                let view = crate::get_generic_wow_match_view_from_id(&*self.db, view_uuid).await?;

                let mut view_cache = self.view_cache.write().await;
                view_cache.put(view_uuid.clone(), view.clone());

                view
            }
        };

        let mut gz = flate2::read::GzDecoder::new(raw_data.as_slice());
        let mut uncompressed_data: Vec<u8> = Vec::new();
        gz.read_to_end(&mut uncompressed_data)?;
        let parsed_data: Vec<RawWoWCombatLogPayload> = match gz.read_to_end(&mut uncompressed_data) {
            Ok(_) => serde_json::from_slice(&uncompressed_data)?,
            Err(_) => serde_json::from_slice(&raw_data)? 
        };

        let mut events: Vec<WoWCombatLogEvent> = Vec::new();
        for mut payload in parsed_data {
            payload.redo_parts();

            if payload.is_finish_token() {
                log::info!("Detect Finish Token for WoW Match View: {}", view_uuid);
            } else {
                let parsed_event = crate::parse_raw_wow_combat_log_payload(view_uuid, match_view.alt_id, match_view.user_id, &match_view.combat_log_state(), &payload)?;
                if let Some(new_event) = parsed_event {
                    events.push(new_event);
                }
            }
        }

        let mut tx = self.db.begin().await?;
        log::info!("Handling {} WoW Combat Log Events", events.len());
        crate::store_wow_combat_log_events(&mut tx, events).await?;
        tx.commit().await?;
        
        Ok(())
    }
    
    pub async fn request_bulk_combat_log_payload_processing(&self, view_uuid: &Uuid, raw_data: Vec<u8>) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.wow_combatlog_queue, serde_json::to_vec(&WowCombatLogTask::Bulk{
            view_uuid: view_uuid.clone(),
            raw_data,
        })?, RABBITMQ_DEFAULT_PRIORITY, -1).await;
        Ok(())
    }
}