use async_trait::async_trait;
use uuid::Uuid;
use crate::{
    SquadOvError,
    rabbitmq::{
        RabbitMqInterface,
        RabbitMqListener,
        RabbitMqConfig,
        RABBITMQ_DEFAULT_PRIORITY,
    },
    elastic::{
        ElasticSearchConfig,
        ElasticSearchClient,
        ElasticSearchDocUpdate,
        vod::{
            ESVodSharing,
            ESVodParentLists,
            ESVodClip,
            ESVodCopy,
        },
        self,
    },
    vod::{
        VodAssociation,
        VodManifest,
        VodMetadata,
        VodTrack,
        RawVodTag,
        db as vdb,
    },
    combatlog::interface::CombatLogInterface,
};
use std::{
    sync::Arc,
};
use sqlx::postgres::{PgPool};
use serde::{Serialize, Deserialize};

const ES_MAX_AGE_SECONDS: i64 = 172800; // 2 day

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ElasticSearchSyncTask {
    SyncVod{
        video_uuid: Vec<Uuid>,
    },
    SyncMatch{
        match_uuid: Uuid,
        user_id: Option<i64>,
    },
    UpdateVodData{
        video_uuid: Uuid,
    },
    UpdateVodSharing{
        video_uuid: Uuid,
    },
    UpdateVodLists{
        video_uuid: Uuid,
    },
    UpdateVodTags{
        video_uuid: Uuid,
    },
    UpdateVodClip{
        video_uuid: Uuid,
    },
    UpdateVodCopies{
        video_uuid: Vec<Uuid>,
    },
}

pub struct ElasticSearchJobInterface {
    es_client: Option<Arc<ElasticSearchClient>>,
    esconfig: Option<ElasticSearchConfig>,
    mqconfig: RabbitMqConfig,
    rmq: Arc<RabbitMqInterface>,
    db: Arc<PgPool>,
    cl_itf: Option<Arc<CombatLogInterface>>,
}

impl ElasticSearchJobInterface {
    pub fn new_producer_only(mqconfig: &RabbitMqConfig, rmq: Arc<RabbitMqInterface>, db: Arc<PgPool>) -> Self {
        Self {
            es_client: None,
            esconfig: None,
            mqconfig: mqconfig.clone(),
            rmq,
            db,
            cl_itf: None,
        }
    }

    pub fn new (es_client: Arc<ElasticSearchClient>, esconfig: &ElasticSearchConfig, mqconfig: &RabbitMqConfig, rmq: Arc<RabbitMqInterface>, db: Arc<PgPool>, cl_itf: Arc<CombatLogInterface>) -> Self {
        Self {
            es_client: Some(es_client),
            esconfig: Some(esconfig.clone()),
            mqconfig: mqconfig.clone(),
            rmq,
            db,
            cl_itf: Some(cl_itf),
        }
    }

    pub async fn request_update_vod_data(&self, video_uuid: Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::UpdateVodData{
            video_uuid,
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn update_vod_data(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        let assoc = vdb::get_vod_association(&*self.db, video_uuid).await?;
        let manifest = vdb::get_vod_manifest(&*self.db, &assoc).await.unwrap_or(VodManifest{
            video_tracks: vec![
                VodTrack{
                    metadata: VodMetadata{
                        video_uuid: video_uuid.clone(),
                        ..VodMetadata::default()
                    },
                    segments: vec![],
                    preview: None,
                }
            ]
        });

        #[derive(Serialize)]
        struct Update {
            vod: VodAssociation,
            manifest: VodManifest,
        }

        let update = ElasticSearchDocUpdate{
            doc: Update{
                vod: assoc,
                manifest,
            }
        };

        self.es_client.as_ref().unwrap().update_document(&self.esconfig.as_ref().unwrap().vod_index_write, video_uuid.to_string().as_str(), update).await?;
        Ok(())
    }

    pub async fn request_update_vod_sharing(&self, video_uuid: Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::UpdateVodSharing{
            video_uuid,
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn update_vod_sharing(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        #[derive(Serialize)]
        struct Update {
            sharing: ESVodSharing,
        }

        let update = ElasticSearchDocUpdate{
            doc: Update{
                sharing: elastic::vod::build_es_vod_document_sharing(&*self.db, video_uuid).await?,
            }
        };

        self.es_client.as_ref().unwrap().update_document(&self.esconfig.as_ref().unwrap().vod_index_write, video_uuid.to_string().as_str(), update).await?;
        Ok(())
    }

    pub async fn request_update_vod_lists(&self, video_uuid: Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::UpdateVodLists{
            video_uuid,
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn update_vod_lists(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        #[derive(Serialize)]
        struct Update {
            lists: ESVodParentLists,
        }

        let assoc = vdb::get_vod_association(&*self.db, video_uuid).await?;
        let update = ElasticSearchDocUpdate{
            doc: Update{
                lists: elastic::vod::build_es_vod_document_lists(&*self.db, video_uuid, &assoc).await?,
            }
        };

        self.es_client.as_ref().unwrap().update_document(&self.esconfig.as_ref().unwrap().vod_index_write, video_uuid.to_string().as_str(), update).await?;
        Ok(())
    }

    pub async fn request_update_vod_tags(&self, video_uuid: Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::UpdateVodTags{
            video_uuid,
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn update_vod_tags(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        #[derive(Serialize)]
        struct Update {
            tags: Vec<RawVodTag>,
        }

        let update = ElasticSearchDocUpdate{
            doc: Update{
                tags: vdb::get_raw_vod_tags(&*self.db, video_uuid).await?,
            }
        };

        self.es_client.as_ref().unwrap().update_document(&self.esconfig.as_ref().unwrap().vod_index_write, video_uuid.to_string().as_str(), update).await?;
        Ok(())
    }

    pub async fn request_update_vod_clip(&self, video_uuid: Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::UpdateVodClip{
            video_uuid,
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn update_vod_clip(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        #[derive(Serialize)]
        struct Update {
            clip: Option<ESVodClip>,
        }

        let update = ElasticSearchDocUpdate{
            doc: Update{
                clip: elastic::vod::build_es_vod_clip(&*self.db, video_uuid).await?,
            }
        };

        self.es_client.as_ref().unwrap().update_document(&self.esconfig.as_ref().unwrap().vod_index_write, video_uuid.to_string().as_str(), update).await?;
        Ok(())
    }

    pub async fn request_update_vod_copies(&self, video_uuid: Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::UpdateVodCopies{
            video_uuid: vec![video_uuid],
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn update_vod_copies(&self, video_uuid: &Uuid) -> Result<(), SquadOvError> {
        #[derive(Serialize)]
        #[serde(rename_all="camelCase")]
        struct Update {
            storage_copies_exact: Option<Vec<ESVodCopy>>,
        }

        let update = ElasticSearchDocUpdate{
            doc: Update{
                storage_copies_exact: Some(elastic::vod::build_es_vod_storage_copies(&*self.db, video_uuid).await?),
            }
        };

        self.es_client.as_ref().unwrap().update_document(&self.esconfig.as_ref().unwrap().vod_index_write, video_uuid.to_string().as_str(), update).await?;
        Ok(())
    }

    pub async fn request_sync_vod(&self, video_uuid: Vec<Uuid>) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.vods
            SET request_sync_elasticsearch = NOW()
            WHERE video_uuid = ANY($1)
            ",
            &video_uuid,
        )
            .execute(&*self.db)
            .await?;

        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::SyncVod{
            video_uuid,
        })?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    pub async fn handle_sync_vod(&self, video_uuid: &[Uuid]) -> Result<(), SquadOvError> {
        // TODO: Actually batch?
        for id in video_uuid {
            if let Ok(doc) = elastic::vod::build_es_vod_document(&*self.db, id, self.cl_itf.as_ref().unwrap().clone()).await {
                self.es_client.as_ref().unwrap().add_or_update_document(&self.esconfig.as_ref().unwrap().vod_index_write, id.to_string().as_str(), serde_json::to_value(doc)?).await?;

                // Actually remember when we last sync'd this data.
                sqlx::query!(
                    "
                    UPDATE squadov.vods
                    SET last_sync_elasticsearch = NOW()
                    WHERE video_uuid = $1
                    ",
                    id,
                )
                    .execute(&*self.db)
                    .await?;
            } else {
                log::warn!("Failed to build ES vod document: {}", id);
            }
        }
        Ok(())
    }

    pub async fn request_sync_match(&self, match_uuid: Uuid, user_id: Option<i64>) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.mqconfig.elasticsearch_queue, serde_json::to_vec(&ElasticSearchSyncTask::SyncMatch{match_uuid: match_uuid.clone(), user_id})?, RABBITMQ_DEFAULT_PRIORITY, ES_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn handle_sync_match(&self, match_uuid: &Uuid, user_id: Option<i64>) -> Result<(), SquadOvError> {
        // The 'sync match' should only happen for games where the match data can possibly come in after
        // the video is processed. This is generally most games so this should trigger a re-sync of the video data
        // to ElasticSearch - all videos with this match UUID should get synced.
        let data: Vec<_> = if let Some(user_id) = user_id {
            sqlx::query!(
                "
                SELECT
                    v.video_uuid
                FROM squadov.matches AS m
                INNER JOIN squadov.vods AS v
                    ON v.match_uuid = m.uuid
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                WHERE m.uuid = $1
                    AND u.id = $2
                ",
                match_uuid,
                user_id,
            )
                .fetch_all(&*self.db)
                .await?
                .into_iter()
                .map(|x| {
                    x.video_uuid
                })
                .collect()
        } else {
            sqlx::query!(
                "
                SELECT
                    v.video_uuid
                FROM squadov.matches AS m
                INNER JOIN squadov.vods AS v
                    ON v.match_uuid = m.uuid
                WHERE m.uuid = $1
                ",
                match_uuid,
            )
                .fetch_all(&*self.db)
                .await?
                .into_iter()
                .map(|x| {
                    x.video_uuid
                })
                .collect()
        };

        if data.is_empty() {
            return Ok(())
        }

        for d in data {
            self.request_sync_vod(vec![d]).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl RabbitMqListener for ElasticSearchJobInterface {
    async fn handle(&self, data: &[u8], queue: &str) -> Result<(), SquadOvError> {
        log::info!("Handle ElasticSearch RabbitMQ Task: {} [{}]", std::str::from_utf8(data).unwrap_or("failure"), queue);
        let task: ElasticSearchSyncTask = serde_json::from_slice(data)?;
        match task {
            ElasticSearchSyncTask::SyncVod{video_uuid} => self.handle_sync_vod(&video_uuid).await?,
            ElasticSearchSyncTask::SyncMatch{match_uuid, user_id} => self.handle_sync_match(&match_uuid, user_id).await?,
            ElasticSearchSyncTask::UpdateVodData{video_uuid} => self.update_vod_data(&video_uuid).await?,
            ElasticSearchSyncTask::UpdateVodSharing{video_uuid} => self.update_vod_sharing(&video_uuid).await?,
            ElasticSearchSyncTask::UpdateVodLists{video_uuid} => self.update_vod_lists(&video_uuid).await?,
            ElasticSearchSyncTask::UpdateVodTags{video_uuid} => self.update_vod_tags(&video_uuid).await?,
            ElasticSearchSyncTask::UpdateVodClip{video_uuid} => self.update_vod_clip(&video_uuid).await?,
            ElasticSearchSyncTask::UpdateVodCopies{video_uuid} => {
                for v in video_uuid {
                    match self.update_vod_copies(&v).await {
                        Ok(_) => (),
                        Err(err) => log::warn!("Failed to update VOD copy: {}", err),
                    }
                }
            }
        };
        Ok(())
    }
}