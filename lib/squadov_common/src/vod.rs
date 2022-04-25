pub mod fastify;
pub mod preview;
pub mod manager;
pub mod db;
pub mod clip;

use async_trait::async_trait;
use serde::{Serialize,Deserialize};
use sqlx::postgres::{PgPool};
use uuid::Uuid;
use std::str;
use std::clone::Clone;
use crate::{
    SquadOvError,
    SquadOvGames,
    rabbitmq::{
        RABBITMQ_DEFAULT_PRIORITY,
        RABBITMQ_HIGH_PRIORITY,
        RabbitMqInterface,
        RabbitMqListener,
    },
    storage::StorageManager,
    elastic::{
        vod::ESVodDocument,
        rabbitmq::ElasticSearchJobInterface,
    },
    matches,
};
use std::sync::{Arc};
use std::io::{self, BufReader};
use tempfile::{NamedTempFile, TempPath};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use md5::{Md5, Digest};

const VOD_MAX_AGE_SECONDS: i64 = 21600; // 6 hours

#[derive(Serialize,Deserialize, Clone)]
pub struct StagedVodClip {
    pub id: i64,
    pub video_uuid: Uuid,
    pub user_id: i64,
    pub start_offset_ms: i64,
    pub end_offset_ms: i64,
    pub create_time: DateTime<Utc>,
    pub execute_time: Option<DateTime<Utc>>,
    pub clip_uuid: Option<Uuid>,
}

#[derive(Serialize,Deserialize, Clone)]
pub struct VodDestination {
    pub url: String,
    pub bucket: String,
    pub session: String,
    pub loc: manager::UploadManagerType,
    pub purpose: manager::UploadPurpose,
}

#[derive(Serialize,Deserialize, Clone)]
pub struct VodThumbnail {
    pub video_uuid: Uuid,
    pub bucket: String,
    pub filepath: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize,Deserialize, Clone, Debug, Default)]
pub struct VodAssociation {
    #[serde(rename = "matchUuid")]
    pub match_uuid: Option<Uuid>,
    #[serde(rename = "userUuid")]
    pub user_uuid: Option<Uuid>,
    #[serde(rename = "videoUuid")]
    pub video_uuid: Uuid,
    #[serde(rename = "startTime")]
    pub start_time: Option<DateTime<Utc>>,
    #[serde(rename = "endTime")]
    pub end_time: Option<DateTime<Utc>>,
    #[serde(rename = "rawContainerFormat")]
    pub raw_container_format: String,
    #[serde(rename = "isClip")]
    pub is_clip: bool,
    #[serde(rename = "isLocal", default)]
    pub is_local: bool,
    pub md5: Option<String>,
    #[serde(skip)]
    pub last_sync_elasticsearch: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub request_sync_elasticsearch: Option<DateTime<Utc>>,
}

#[derive(Serialize,Deserialize,Clone,Debug)]
pub struct VodMetadata {
    #[serde(rename = "videoUuid", default)]
    pub video_uuid: Uuid,
    #[serde(rename = "resX")]
    pub res_x: i32,
    #[serde(rename = "resY")]
    pub res_y: i32,
    pub fps: i32,

    #[serde(rename = "minBitrate")]
    pub min_bitrate: i64,
    #[serde(rename = "avgBitrate")]
    pub avg_bitrate: i64,
    #[serde(rename = "maxBitrate")]
    pub max_bitrate: i64,
    pub bucket: String,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,

    pub id: String,
    #[serde(skip)]
    pub has_fastify: bool,
    #[serde(skip)]
    pub has_preview: bool,
}

impl Default for  VodMetadata {
    fn default() -> Self {
        Self {
            video_uuid: Uuid::new_v4(),
            res_x: 0,
            res_y: 0,
            fps: 0,
            min_bitrate: 0,
            avg_bitrate: 0,
            max_bitrate: 0,
            bucket: String::new(),
            id: String::new(),
            session_id: None,
            has_fastify: false,
            has_preview: false,
        }
    }
}

#[derive(Deserialize,Debug)]
pub struct VodSegmentId {
    pub video_uuid: Uuid,
    pub quality: String,
    pub segment_name: String
}

impl VodSegmentId {
    fn get_path_parts(&self) -> Vec<String> {
        vec![self.video_uuid.to_string(), self.quality.clone(), self.segment_name.clone()]
    }

    fn get_fname(&self) -> String {
        self.get_path_parts().join("/")
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RawVodTag {
    pub video_uuid: Uuid,
    pub tag_id: i64,
    pub tag: String,
    pub user_id: i64,
    pub tm: DateTime<Utc>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all="camelCase")]
pub struct VodTag {
    pub video_uuid: Uuid,
    // The text of the tag
    pub tag: String,
    pub tag_id: i64,
    // How many people applied this same exact tag
    pub count: i64,
    // Whether or not the person doing the query applied this tag
    pub is_self: bool,
}

pub fn condense_raw_vod_tags(tags: Vec<RawVodTag>, self_user_id: i64) -> Vec<VodTag> {
    let mut store: HashMap<String, VodTag> = HashMap::new();
    for t in tags {
        if !store.contains_key(&t.tag) {
            store.insert(t.tag.clone(), VodTag{
                video_uuid: t.video_uuid.clone(),
                tag: t.tag.clone(),
                tag_id: t.tag_id,
                count: 0,
                is_self: false,
            });
        }

        if let Some(mt) = store.get_mut(&t.tag) {
            mt.count += 1;
            mt.is_self |= t.user_id == self_user_id;
        }
    }
    store.values().cloned().collect()
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct VodClip {
    pub clip: VodAssociation,
    pub manifest: VodManifest,
    pub title: String,
    pub description: String,
    pub clipper: String,
    pub game: SquadOvGames,
    pub tm: DateTime<Utc>,
    pub views: i64,
    pub reacts: i64,
    pub comments: i64,
    pub favorite_reason: Option<String>,
    pub is_watchlist: bool,
    pub access_token: Option<String>,
    pub tags: Vec<VodTag>,
    pub published: bool,
}

#[derive(Default)]
pub struct VodClipReactStats {
    pub video_uuid: Uuid,
    pub views: i64,
    pub reacts: i64,
    pub comments: i64,
}

#[derive(Serialize,Deserialize,Clone)]
#[serde(rename_all="camelCase")]
pub struct ClipReact {
}

#[derive(Serialize,Deserialize,Clone)]
#[serde(rename_all="camelCase")]
pub struct ClipComment {
    pub id: i64,
    pub clip_uuid: Uuid,
    pub username: String,
    pub comment: String,
    pub tm: DateTime<Utc>,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct VodSegment {
    pub uri: String,
    pub duration: f32,
    #[serde(rename="segmentStart")]
    pub segment_start: f32,
    #[serde(rename="mimeType")]
    pub mime_type: String,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct VodTrack {
    pub metadata: VodMetadata,
    pub segments: Vec<VodSegment>,
    pub preview: Option<String>,
}

#[derive(Serialize,Deserialize,Debug, Clone)]
pub struct VodManifest {
    #[serde(rename="videoTracks")]
    pub video_tracks: Vec<VodTrack>
}

impl Default for VodManifest {
    fn default() -> Self {
        return Self{
            video_tracks: Vec::new()
        }
    }
}

pub struct VodProcessingInterface {
    queue: String,
    rmq: Arc<RabbitMqInterface>,
    db: Arc<PgPool>,
    vod: Arc<StorageManager<Arc<dyn manager::VodManager + Send + Sync>>>,
    es_itf: Arc<ElasticSearchJobInterface>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum VodProcessingTask {
    Process{
        vod_uuid: Uuid,
        session_id: Option<String>,
        id: Option<String>,
    },
    GeneratePreview{
        vod_uuid: Uuid,
    },
    GenerateThumbnail{
        vod_uuid: Uuid,
    },
    GenerateStagedClip{
        request: StagedVodClip,
    }
}

#[async_trait]
impl RabbitMqListener for VodProcessingInterface {
    async fn handle(&self, data: &[u8], _queue: &str) -> Result<(), SquadOvError> {
        log::info!("Handle VOD Task: {}", std::str::from_utf8(data).unwrap_or("failure"));
        let task: VodProcessingTask = serde_json::from_slice(data)?;
        match task {
            VodProcessingTask::Process{vod_uuid, id, session_id} => self.process_vod(
                &vod_uuid,
                &id.unwrap_or(String::from("source")),
                session_id.as_ref(),
            ).await?,
            VodProcessingTask::GeneratePreview{vod_uuid} => self.generate_preview(&vod_uuid).await?,
            VodProcessingTask::GenerateThumbnail{vod_uuid} => self.generate_thumbnail(&vod_uuid).await?,
            VodProcessingTask::GenerateStagedClip{request} => self.generate_staged_clip(&request).await?,
        };
        Ok(())
    }
}

impl VodProcessingInterface {
    pub fn new(queue: &str, rmq: Arc<RabbitMqInterface>, db: Arc<PgPool>, vod: Arc<StorageManager<Arc<dyn manager::VodManager + Send + Sync>>>, es_itf: Arc<ElasticSearchJobInterface>) -> Self {
        Self {
            queue: String::from(queue),
            rmq,
            db,
            vod,
            es_itf,
        }
    }

    pub async fn request_vod_processing(&self, vod_uuid: &Uuid, id: &str, session_id: Option<String>, high_priority: bool) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.queue, serde_json::to_vec(&VodProcessingTask::Process{
            vod_uuid: vod_uuid.clone(),
            session_id,
            id: Some(id.to_string()),
        })?, if high_priority { RABBITMQ_HIGH_PRIORITY } else { RABBITMQ_DEFAULT_PRIORITY }, VOD_MAX_AGE_SECONDS).await;
        Ok(())
    }

    pub async fn request_generate_preview(&self, vod_uuid: & Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.queue, serde_json::to_vec(&VodProcessingTask::GeneratePreview{
            vod_uuid: vod_uuid.clone(),
        })?, RABBITMQ_DEFAULT_PRIORITY, VOD_MAX_AGE_SECONDS).await;
        Ok(())
    }

    pub async fn request_generate_thumbnail(&self, vod_uuid: & Uuid) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.queue, serde_json::to_vec(&VodProcessingTask::GenerateThumbnail{
            vod_uuid: vod_uuid.clone(),
        })?, RABBITMQ_DEFAULT_PRIORITY, VOD_MAX_AGE_SECONDS).await;
        Ok(())
    }

    pub async fn request_generate_staged_clip(&self, request: &StagedVodClip) -> Result<(), SquadOvError> {
        self.rmq.publish(&self.queue, serde_json::to_vec(&VodProcessingTask::GenerateStagedClip{
            request: request.clone(),
        })?, RABBITMQ_DEFAULT_PRIORITY, VOD_MAX_AGE_SECONDS).await;
        Ok(())
    }

    async fn download_vod_locally(&self, vod_uuid: &Uuid, context: &str) -> Result<(VodAssociation, VodMetadata, TempPath), SquadOvError> {
        log::info!("[{}] Get VOD Association {}", context, vod_uuid);
        let vod = db::get_vod_association(&*self.db, vod_uuid).await?;

        log::info!("[{}] Generate Preview for VOD {}", context, vod_uuid);
        let metadata = db::get_vod_metadata(&*self.db, vod_uuid, "source").await?;

        log::info!("[{}] Get VOD Manager {}", context, vod_uuid);
        let manager = self.vod.get_bucket(&metadata.bucket).await.ok_or(SquadOvError::InternalError(format!("Invalid bucket: {}", &metadata.bucket)))?;

        let input_filename = NamedTempFile::new()?.into_temp_path();
        log::info!("[{}] Download VOD - {}", context, vod_uuid);
        let source_segment_id = VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("fastify.mp4"),
        };
        manager.download_vod_to_path(&source_segment_id, &input_filename).await?;
        Ok((vod, metadata, input_filename))
    }

    pub async fn generate_preview(&self, vod_uuid: &Uuid) -> Result<(), SquadOvError> {
        let (vod, metadata, input_filename) = self.download_vod_locally(vod_uuid, "Preview").await?;

        let preview_filename = NamedTempFile::new()?.into_temp_path();
        // Get VOD length in seconds - we use this to manually determine where to clip.
        let length_seconds = vod.end_time.unwrap_or(Utc::now()).signed_duration_since(vod.start_time.unwrap_or(Utc::now())).num_seconds();

        log::info!("[Preview] Generate Preview Mp4 - {}", vod_uuid);
        preview::generate_vod_preview(input_filename.as_os_str().to_str().ok_or(SquadOvError::BadRequest)?, &preview_filename, length_seconds).await?;

        log::info!("[Preview] Upload Preview VOD - {}", vod_uuid);
        let manager = self.vod.get_bucket(&metadata.bucket).await.ok_or(SquadOvError::InternalError(format!("Invalid bucket: {}", &metadata.bucket)))?;
        manager.upload_vod_from_file(&VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("preview.mp4"),
        }, &preview_filename, manager::StorageType::Hot).await?;

        log::info!("[Preview] Process VOD TX (Begin) - {}", vod_uuid);
        let mut tx = self.db.begin().await?;

        log::info!("[Preview] Mark DB Preview (Query) - {}", vod_uuid);
        db::mark_vod_with_preview(&mut tx, vod_uuid).await?;
        log::info!("[Preview] Process VOD TX (Commit) - {}", vod_uuid);
        tx.commit().await?;

        log::info!("[Preview] Dispatch ES Update - {}", vod_uuid);
        self.es_itf.request_update_vod_data(vod_uuid.clone()).await?;
        Ok(())
    }

    pub async fn generate_staged_clip(&self, request: &StagedVodClip) -> Result<(), SquadOvError> {
        log::info!("[Clip] Downloading VOD {}", request.id);
        let (vod, metadata, input_filename) = self.download_vod_locally(&request.video_uuid, "Clip").await?;

        log::info!("[Clip] Get VOD Manager {}", request.id);
        let manager = self.vod.get_bucket(&metadata.bucket).await.ok_or(SquadOvError::InternalError(format!("Invalid bucket: {}", &metadata.bucket)))?;

        log::info!("[Clip] Generating Clip {}", request.id);
        let clip_filename = NamedTempFile::new()?.into_temp_path();
        clip::generate_clip(&input_filename.as_os_str().to_str().ok_or(SquadOvError::BadRequest)?, &clip_filename, request.start_offset_ms, request.end_offset_ms).await?;

        log::info!("[Clip] Computing VOD MD5 - {}", request.id);
        let md5_hash = {
            let mut file = std::fs::File::open(&clip_filename)?;
            let mut hasher = Md5::default();
            let _n = io::copy(&mut file, &mut hasher);
            base64::encode(hasher.finalize())
        };

        log::info!("[Clip] - Uploading {}", request.id);
        let clip_uuid = Uuid::new_v4();
        let clip_id = VodSegmentId{
            video_uuid: clip_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("fastify.mp4"),
        };
        manager.upload_vod_from_file(&clip_id, &clip_filename, manager::StorageType::Hot).await?;

        log::info!("[Clip] Inferring Metadata {}", request.id);
        let mut clip_metadata = metadata.clone();
        clip_metadata.session_id = None;
        clip_metadata.id = String::from("source");
        clip_metadata.video_uuid = clip_uuid.clone();

        log::info!("[Clip] TX (Begin) - {}", request.id);
        let mut tx = self.db.begin().await?;

        log::info!("[Clip] Reserving Clip UUID - {}", request.id);
        db::reserve_vod_uuid(&mut tx, &clip_uuid, "mp4", request.user_id, true).await?;

        log::info!("[Clip] Creating Clip (DB) - {}", request.id);
        db::create_clip(
            &mut tx,
            &clip_uuid,
            &request.video_uuid,
            request.user_id,
            "Instantly Clipped by SquadOV!",
            "",
            matches::get_game_for_match(&*self.db, &vod.match_uuid.ok_or(SquadOvError::BadRequest)?).await?,
            false,
        ).await?;

        log::info!("[Clip] Associating VOD (DB) - {}", request.id);
        db::associate_vod(&mut tx, &VodAssociation{
            match_uuid: vod.match_uuid.clone(),
            user_uuid: vod.user_uuid.clone(),
            video_uuid: clip_uuid.clone(),
            start_time: vod.start_time.map(|x| { x + chrono::Duration::milliseconds(request.start_offset_ms) }),
            end_time: vod.start_time.map(|x| { x + chrono::Duration::milliseconds(request.end_offset_ms) }),
            raw_container_format: "mp4".to_string(),
            is_clip: true,
            is_local: false,
            md5: None,
            last_sync_elasticsearch: None,
            request_sync_elasticsearch: None,
        }).await?;

        log::info!("[Clip] Add Video Metadata - {}", request.id);
        db::bulk_add_video_metadata(&mut tx, &clip_uuid, &[clip_metadata]).await?;

        log::info!("[Clip] Mark Fastify (Query) - {}", request.id);
        db::mark_vod_as_fastify(&mut tx, &clip_uuid).await?;

        log::info!("[Clip] Store VOD MD5 - {}", request.id);
        db::store_vod_md5(&mut tx, &clip_uuid, &md5_hash).await?;

        log::info!("[Clip] Dispatch Jobs - {}", request.id);
        self.request_generate_preview(&clip_uuid).await?;
        self.request_generate_thumbnail(&clip_uuid).await?;

        log::info!("[Clip] Mark Executed - {}", request.id);
        db::mark_staged_clip_executed(&mut tx, request.id, &clip_uuid).await?;

        log::info!("[Clip] TX (Commit) - {}", request.id);
        tx.commit().await?;

        log::info!("[Clip] Dispatch ElasticSearch - {}", request.id);
        self.es_itf.request_sync_vod(vec![clip_uuid.clone()]).await?;
        Ok(())
    }

    pub async fn generate_thumbnail(&self, vod_uuid: &Uuid) -> Result<(), SquadOvError> {
        let (vod, metadata, input_filename) = self.download_vod_locally(vod_uuid, "Thumbnail").await?;

        let thumbnail_filename = NamedTempFile::new()?.into_temp_path();
        // Get VOD length in seconds - we use this to manually determine where to clip.
        let length_seconds = vod.end_time.unwrap_or(Utc::now()).signed_duration_since(vod.start_time.unwrap_or(Utc::now())).num_seconds();

        log::info!("[Thumbnail] Generate Thumbnail - {}", vod_uuid);
        preview::generate_vod_thumbnail(input_filename.as_os_str().to_str().ok_or(SquadOvError::BadRequest)?, &thumbnail_filename, length_seconds).await?;

        log::info!("[Thumbnail] Upload Thumbnail - {}", vod_uuid);
        let manager = self.vod.get_bucket(&metadata.bucket).await.ok_or(SquadOvError::InternalError(format!("Invalid bucket: {}", &metadata.bucket)))?;
        let thumbnail_id = VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("thumbnail.jpg"),
        };
        manager.upload_vod_from_file(&thumbnail_id, &thumbnail_filename, manager::StorageType::Hot).await?;

        log::info!("[Thumbnail] Process VOD TX (Begin) - {}", vod_uuid);
        let mut tx = self.db.begin().await?;

        log::info!("[Thumbnail] Add DB Thumbnail (Query) - {}", vod_uuid);
        {
            let file = std::fs::File::open(&thumbnail_filename)?;
            let image = image::io::Reader::with_format(BufReader::new(file), image::ImageFormat::Jpeg);
            let thumbnail_dims = image.into_dimensions()?;
            db::add_vod_thumbnail(&mut tx, vod_uuid, &metadata.bucket, &thumbnail_id, thumbnail_dims.0 as i32, thumbnail_dims.1 as i32).await?;
        }

        log::info!("[Thumbnail] Check if VOD is Public - {}", vod_uuid);
        if db::check_if_vod_public(&*self.db, vod_uuid).await? {
            log::info!("[Thumbnail] Setting Thumbnail as Public - {}", vod_uuid);
            manager.make_segment_public(&thumbnail_id).await?;
        }

        log::info!("[Thumbnail] Process VOD TX (Commit) - {}", vod_uuid);
        tx.commit().await?;

        log::info!("[Thumbnail] Dispatch ES Update - {}", vod_uuid);
        self.es_itf.request_update_vod_data(vod_uuid.clone()).await?;
        Ok(())
    }

    pub async fn process_vod(&self, vod_uuid: &Uuid, id: &str, session_id: Option<&String>) -> Result<(), SquadOvError> {
        log::info!("[Fastify] Start Processing VOD {} [{:?}]", vod_uuid, session_id);

        log::info!("[Fastify] Get VOD Association");
        let vod = db::get_vod_association(&*self.db, vod_uuid).await?;

        log::info!("[Fastify] Get Container Extension");
        let raw_extension = container_format_to_extension(&vod.raw_container_format);
        let source_segment_id = VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: format!("video.{}", &raw_extension),
        };

        // Need to grab the metadata so we know where this VOD was stored.
        log::info!("[Fastify] Get VOD Metadata");
        let metadata = db::get_vod_metadata(&*self.db, vod_uuid, id).await?;

        // Grab the appropriate VOD manager. The manager should already exist!
        log::info!("[Fastify] Get VOD Manager");
        let manager = self.vod.get_bucket(&metadata.bucket).await.ok_or(SquadOvError::InternalError(format!("Invalid bucket: {}", &metadata.bucket)))?;

        // Note that we can only proceed with "fastifying" the VOD if the entire VOD has been uploaded.
        // We can query GCS's XML API to determine this. If the GCS Session URI is not provided then
        // we assume that the file has already been fully uploaded. If the file hasn't been fully uploaded
        // then we want to defer taking care of this task until later.
        if let Some(session) = session_id {
            log::info!("Checking Segment Upload Finished");
            if !manager.is_vod_session_finished(&session).await? {
                log::info!("Defer Fastifying {:?}", vod_uuid);
                return Err(SquadOvError::Defer(1000));
            }
        } 

        // We only do the "fastify" process here. Other jobs get farmed out to separate tasks
        // to ensure that the "processing" doesn't fail if the thumbnail failed to generate for example.
        // 1) Download the VOD to disk using the VOD manager (I think this gets us
        //    faster DL speed than using FFMPEG directly).
        // 2) Convert the video using the vod.fastify module. This gets us a VOD
        //    that has the faststart flag.
        // 3) Upload the processed video using the VOD manager.
        // 4) Mark the video as being "fastified" (I really need a better word).
        log::info!("[Fastify] Generate Input Temp File");
        let input_filename = NamedTempFile::new()?.into_temp_path();
        log::info!("[Fastify] Download VOD - {}", vod_uuid);
        
        manager.download_vod_to_path(&source_segment_id, &input_filename).await?;

        let fastify_filename = NamedTempFile::new()?.into_temp_path();

        log::info!("[Fastify] Fastify Mp4 - {}", vod_uuid);
        fastify::fastify_mp4(input_filename.as_os_str().to_str().ok_or(SquadOvError::BadRequest)?, &vod.raw_container_format, &fastify_filename).await?;

        log::info!("[Fastify] Upload Fastify VOD - {}", vod_uuid);
        let fastify_segment = VodSegmentId{
            video_uuid: vod_uuid.clone(),
            quality: String::from("source"),
            segment_name: String::from("fastify.mp4"),
        };
        manager.upload_vod_from_file(&fastify_segment, &fastify_filename, manager::StorageType::Cold).await?;

        log::info!("[Fastify] Process VOD TX (Begin) - {}", vod_uuid);
        let mut tx = self.db.begin().await?;
        log::info!("[Fastify] Mark DB Fastify (Query) - {}", vod_uuid);
        db::mark_vod_as_fastify(&mut tx, vod_uuid).await?;

        log::info!("[Fastify] Computing VOD MD5 - {}", vod_uuid);
        let md5_hash = {
            let mut file = std::fs::File::open(&fastify_filename)?;
            let mut hasher = Md5::default();
            let _n = io::copy(&mut file, &mut hasher);
            base64::encode(hasher.finalize())
        };

        log::info!("[Fastify] Store VOD MD5 - {}", vod_uuid);
        db::store_vod_md5(&mut tx, vod_uuid, &md5_hash).await?;

        log::info!("[Fastify] Process VOD TX (Commit) - {}", vod_uuid);
        tx.commit().await?;

        log::info!("[Fastify] Delete Source VOD - {}", vod_uuid);
        match manager.delete_vod(&source_segment_id).await {
            Ok(()) => (),
            Err(err) => log::warn!("Failed to delete source VOD: {}", err),
        };

        log::info!("[Fastify] Check if VOD is Public - {}", vod_uuid);
        if db::check_if_vod_public(&*self.db, vod_uuid).await? {
            log::info!("Setting Fastify as Public - {}", vod_uuid);
            manager.make_segment_public(&fastify_segment).await?;
        }

        log::info!("[Fastify] Dispatch ES Update - {}", vod_uuid);
        self.es_itf.request_update_vod_data(vod_uuid.clone()).await?;

        log::info!("[Fastify] Getting Staged Clips - {}", vod_uuid);
        let staged_clips = db::get_staged_clips_for_vod(&*self.db, vod_uuid).await?;

        log::info!("[Fastify] Dispatch Jobs - {}", vod_uuid);
        self.request_generate_preview(vod_uuid).await?;
        self.request_generate_thumbnail(vod_uuid).await?;
        for sc in staged_clips {
            self.request_generate_staged_clip(&sc).await?;
        }

        log::info!("[Fastify] Finish Fastifying {:?}", vod_uuid);
        Ok(())
    }
}

pub fn container_format_to_extension(container_format: &str) -> String {
    match container_format {
        "mpegts" => String::from("ts"),
        _ => String::from("mp4")
    }
}

pub fn container_format_to_mime_type(container_format: &str) -> String {
    match container_format {
        "mpegts" => String::from("video/mp2t"),
        _ => String::from("video/mp4")
    }
}

pub fn vod_document_to_vod_clip_for_user(doc: ESVodDocument, user_id: i64) -> Option<VodClip> {
    if let Some(c) = doc.clip.as_ref() {
        let fav = doc.find_favorite_reason(user_id);
        let watchlist = doc.is_on_user_watchlist(user_id);
        Some(VodClip {
            clip: doc.vod.clone(),
            manifest: doc.manifest.clone(),
            title: c.title.clone(),
            description: c.description.clone(),
            clipper: doc.owner.username,
            game: doc.data.game,
            tm: doc.vod.end_time.unwrap_or(Utc::now()),
            views: 0,
            reacts: 0,
            comments: 0,
            favorite_reason: fav,
            is_watchlist: watchlist,
            access_token: None,
            tags: condense_raw_vod_tags(doc.tags, user_id),
            published: c.published,
        })
    } else {
        None
    }
}