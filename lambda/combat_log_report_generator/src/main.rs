#[macro_use]
extern crate lazy_static;

use structopt::StructOpt;
use lambda_runtime::{handler_fn, Context};
use serde::Deserialize;
use serde_json::{Value};
use std::sync::RwLock;
use squadov_common::{
    SquadOvError,
    encode,
    combatlog::{
        self,
        CombatLogReportGenerator,
        CombatLogReportContainer,
        CombatLogReportType,
        CombatLog,
        CombatLogReport,
    },
    wow::{
        reports::WowReportsGenerator,
    },
    ff14::reports::Ff14ReportsGenerator,
    aws::s3,
    rabbitmq::{RabbitMqInterface, RabbitMqConfig},
    elastic::{
        rabbitmq::ElasticSearchJobInterface,
    },
};
use regex::Regex;
use std::{
    io::{
        Write,
        Seek,
        BufReader,
        BufRead,
    },
    str::FromStr,
    sync::Arc,
    fs::File,
};
use rusoto_core::{Region, HttpClient};
use rusoto_s3::{
    S3Client,
    S3,
    ListObjectsV2Request,
    GetObjectRequest,
    DeleteObjectsRequest,
    Delete,
    ObjectIdentifier,
};
use rusoto_secretsmanager::{
    SecretsManagerClient,
    SecretsManager,
    GetSecretValueRequest,
};
use chrono::{DateTime, Utc};
use tokio::io::AsyncReadExt;
use sqlx::{
    ConnectOptions,
    postgres::{PgPool, PgPoolOptions, PgConnectOptions},
};
use rusoto_credential::{ProfileProvider};

#[derive(Deserialize)]
struct Payload {
    #[serde(rename="Records")]
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    s3: S3Record,
}

#[derive(Deserialize)]
struct S3Record {
    bucket: S3Bucket,
    object: S3Object,
}

#[derive(Deserialize)]
struct S3Bucket {
    name: String,
}

#[derive(Deserialize)]
struct S3Object {
    key: String,
}

struct SharedClient {
    s3: Arc<S3Client>,
    efs_directory: String,
    pool: Arc<PgPool>,
    es_itf: Arc<ElasticSearchJobInterface>,
}

impl SharedClient {
    async fn load_merge_combat_log_data_to_disk(&self, bucket: &str, form: &str, partition: &str, need_merge: bool) -> Result<File, SquadOvError> {
        // The data in S3 will be split into multiple compressed files so we want to merge them all into one uncompressed file for us to deal with while processing.
        let parsed_object_prefix = format!("form={}/partition={}/", form, partition);
        log::info!("Load Separated Combat Log: {}/{}", bucket, parsed_object_prefix);

        // We need to first get all the files with the given prefix and then we need to sort.
        // We want to sort by date of file creation since Firehose *should* dump all the data out in its
        // current buffer into the same file for each partition.
        let mut continuation_token: Option<String> = None;

        #[derive(Default, Debug)]
        struct S3Key {
            key: String,
            last_modified: Option<DateTime<Utc>>,
        }

        let mut available_keys: Vec<S3Key> = vec![];
        loop {
            let req = ListObjectsV2Request{
                bucket: String::from(bucket),
                continuation_token: continuation_token.clone(),
                prefix: Some(parsed_object_prefix.clone()),
                delimiter: Some(String::from("/")),
                ..ListObjectsV2Request::default()
            };

            log::info!("...Listing Objects.");
            let resp = self.s3.list_objects_v2(req).await?;

            if let Some(objects) = resp.contents {
                for obj in objects {
                    let mut obj_key = S3Key::default();

                    if let Some(key) = obj.key {
                        obj_key.key = key;
                    } else {
                        log::warn!("Skipping over object because it doesn't have a key: {:?}", &obj);
                        continue;
                    }

                    obj_key.last_modified = Some(obj.last_modified
                        .map(|x| {
                            DateTime::parse_from_rfc3339(&x)
                        })
                        .map_or(Ok(None), |r| r.map(Some))?
                        .map(|x| {
                            DateTime::from(x)
                        })
                        .unwrap_or(Utc::now()));
                    available_keys.push(obj_key);
                }
            }

            if let Some(is_trunc) = resp.is_truncated {
                if is_trunc {
                    continuation_token = resp.next_continuation_token;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Sort in order of ascending last modified.
        log::info!("...Sorting Objects.");
        available_keys.sort_by(|a, b| {
            let a_mod = a.last_modified.as_ref().unwrap();
            let b_mod = b.last_modified.as_ref().unwrap();
            a_mod.partial_cmp(b_mod).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Download files one by one, uncompress, and merge them into the temp file.
        let mut file = tempfile::tempfile_in(&self.efs_directory)?;

        if need_merge {
            log::info!("...Performing Merge.");
            for key in &available_keys {
                let req = GetObjectRequest{
                    bucket: String::from(bucket),
                    key: key.key.clone(),
                    ..GetObjectRequest::default()
                };

                let resp = self.s3.get_object(req).await?;

                let mut compressed_data: Vec<u8> = Vec::new();
                if let Some(body) = resp.body {
                    let mut body = body.into_async_read();
                    body.read_to_end(&mut compressed_data).await?;
                } else {
                    log::error!("Invalid body for object: {}/{:?}", bucket, &key);
                    continue;
                }

                log::info!("Decompressing: {}/{} - {} bytes", bucket, &key.key, compressed_data.len());
                let mut decoder = flate2::write::GzDecoder::new(&file);
                decoder.write_all(&mut compressed_data)?;
                decoder.finish()?;

                // Note that we do not need to add an extra new line at the end because the parsed data
                // should already contain that new line. Note that our report generation will need to be resilient
                // to new lines.
            }

            log::info!("Decoded Merged File Size: {}", file.metadata()?.len());

            // In addition to creating the merged file for our own consumption.
            // We should also upload back to S3 so we don't have to do this merge again.
            // As an added benefit of that, we should theoretically have a larger file in the end,
            // which will allow us to lifecycle the resulting file into cheaper storage.
            {
                let compressed_file = tempfile::tempfile_in(&self.efs_directory)?;
                let mut encoder = flate2::write::GzEncoder::new(compressed_file, flate2::Compression::default());

                file.seek(std::io::SeekFrom::Start(0))?;
                std::io::copy(&mut file, &mut encoder)?;

                let compressed_file = encoder.finish()?;
                let compressed_size = compressed_file.metadata()?.len() as usize;
                log::info!("Compressed Merged File Size: {}", compressed_size);

                s3::s3_multipart_upload_data(
                    self.s3.clone(),
                    tokio::fs::File::from_std(compressed_file),
                    compressed_size,
                    bucket,
                    &format!("{}completed_merged_{}.gz",parsed_object_prefix, Utc::now().timestamp_millis()),
                ).await?;
            }

            file.seek(std::io::SeekFrom::Start(0))?;
        }

        // Also cleanup all the files we retrieved - we shouldn't ever need to use them again.
        // If need_merge is true, then we would've uploaded a merged file that's sufficient for future purposes.
        // If need_merge is false, then it isn't necessary to actually keep the files we found.
        let delete_chunks: Vec<_> = available_keys.chunks(1000).collect();
        log::info!("...Deleting Chunks: {}.", delete_chunks.len());
        for chunk in delete_chunks {
            let req = DeleteObjectsRequest{
                bucket: bucket.to_string(),
                delete: Delete{
                    objects: chunk.into_iter().map(|x| {
                        ObjectIdentifier{
                            key: x.key.clone(),
                            version_id: None,
                        }
                    }).collect(),
                    ..Delete::default()
                },
                ..DeleteObjectsRequest::default()
            };

            self.s3.delete_objects(req).await?;
        }

        Ok(file)
    }

    async fn create_report_generator<'a>(&self, game: &'a str, id: &'a str, work_dir: &'a str) -> Result<Arc<RwLock<dyn CombatLogReportGenerator + Send + Sync>>, SquadOvError> {
        let report = sqlx::query_as!(
            CombatLog,
            "
            SELECT *
            FROM squadov.combat_logs
            WHERE partition_id = $1
            ",
            &format!("{}_{}", game, id)
        )
            .fetch_one(&*self.pool)
            .await?;

        log::info!("Create Report Generator For Game: {}", game);
        let gen: Arc<RwLock<dyn CombatLogReportGenerator + Send + Sync>> = match game {
            "ff14" => Arc::new(RwLock::new(CombatLogReportContainer::new(
                // TODO: Pull actual start time from database
                Ff14ReportsGenerator::new(report.start_time)?
            ))),
            "wow" => Arc::new(RwLock::new(CombatLogReportContainer::new(
                WowReportsGenerator::new(report, self.pool.clone())?
            ))),
            _ => {
                log::error!("Unsupported game for combat log generation: {}", &game);
                return Err(SquadOvError::BadRequest);
            },
        };
        
        {
            log::info!("Initialize Work Dir: {}", work_dir);
            let mut gen = gen.write()?;
            gen.initialize_work_dir(work_dir)?;
        }
        Ok(gen)
    }

    async fn generate_reports<'a>(&self, gen: Arc<RwLock<dyn CombatLogReportGenerator + Send + Sync>>, mut file: File) -> Result<Vec<Arc<dyn CombatLogReport + Send + Sync>>, SquadOvError> {
        // Seek to beginning of file just because we were previously writing to the file so the stream offset is probably at the end.
        file.seek(std::io::SeekFrom::Start(0))?;

        log::info!("Read Parsed Report File");
        let reader = BufReader::new(file);

        {
            let mut gen = gen.write()?;
            for ln in reader.lines() {
                if let Ok(ln) = ln {
                    let data = ln.trim();
                    if data.is_empty() {
                        continue;
                    }
                    log::trace!("Handle: {:?}", &data);
                    gen.handle(data)?;
                }
            }
        }

        tokio::task::spawn_blocking(move || {
            log::info!("Finalize Reports");
            let mut gen = gen.write()?;
            gen.finalize()?;
            log::info!("Return Reports");
            Ok::<_, SquadOvError>(gen.get_reports())
        }).await??
    }

    async fn handle_s3_data(&self, data: S3Record) -> Result<(), SquadOvError> {
        // The key is in the form:
        // form=Flush/partition=KEY
        // So we need to parse out the partition key to get 1) the game and 2) the unique ID since it's in the form: GAME_ID.
        // We can dispatch the report generation task based off of the game that we parsed out.
        lazy_static! {
            static ref RE_KEY: Regex = Regex::new(r"form=(?P<form>.*)/partition=(?P<partition>.*)/").unwrap();
            static ref RE_MATCH: Regex = Regex::new(r"(?P<game>.*)_(?P<id>.*)").unwrap();
        }

        let decoded_key = encode::url_decode(&data.object.key)?;
        if let Some(key_cap) = RE_KEY.captures(&decoded_key) {
            let form = key_cap.name("form").map(|x| { String::from(x.as_str()) }).unwrap_or(String::new());
            let partition = key_cap.name("partition").map(|x| { String::from(x.as_str()) }).unwrap_or(String::new());

            // Sanity check the form field first - need to make sure it's "Flush" since that's the only thing we care about.
            if form != "Flush" {
                log::error!("Incorrect form for Combat log report generation: {}", &data.object.key);
                return Err(SquadOvError::BadRequest);
            }

            if let Some(match_cap) = RE_MATCH.captures(&partition) {
                let game = match_cap.name("game").map(|x| { String::from(x.as_str()) }).unwrap_or(String::new());
                let id = match_cap.name("id").map(|x| { String::from(x.as_str()) }).unwrap_or(String::new());

                log::info!("Found Match: {} - {}", &game, &id);
                let all_reports = self.generate_reports(
                    self.create_report_generator(&game, &id, &self.efs_directory).await?,
                    self.load_merge_combat_log_data_to_disk(&data.bucket.name, "Parsed", &partition, true).await?
                ).await?;

                // There's two types of reports - static and "dynamic". Dynamic reports technicaly aren't really "dynamic";
                // rather, they're just stored in the database rather than in S3.
                log::info!("Store All Reports: {}", all_reports.len());
                combatlog::store_static_combat_log_reports(all_reports.clone(), &data.bucket.name, &partition, self.s3.clone()).await?;

                let mut tx = self.pool.begin().await?;
                for r in all_reports {
                    if r.report_type() == CombatLogReportType::Dynamic {
                        r.store_dynamic_report(&mut tx).await?;
                    }
                }
                tx.commit().await?;

                log::info!("Merge and Cleanup Raw Logs");
                match self.load_merge_combat_log_data_to_disk(&data.bucket.name, "Raw", &partition, true).await {
                    Ok(_) => (),
                    Err(err) => log::warn!("Failed to merge raw data: {}.", err),
                };

                log::info!("Merge and Cleanup Flush Logs");
                match self.load_merge_combat_log_data_to_disk(&data.bucket.name, "Flush", &partition, false).await {
                    Ok(_) => (),
                    Err(err) => log::warn!("Failed to merge flush data: {}.", err),
                };

                log::info!("Sending ES Document Generation");
                match game.as_str() {
                    "wow" => {
                        let match_view = squadov_common::get_generic_wow_match_view_from_combat_log_id(&*self.pool, &partition).await?;
                        if let Some(match_uuid) = match_view.match_uuid {
                            log::info!("...Sending Sync Match VOD: {} {}", &match_uuid, match_view.user_id);
                            self.es_itf.request_sync_match(match_uuid.clone(), Some(match_view.user_id)).await?;
                        }
                    },
                    _ => {
                        log::error!("Invalid game for ES document generation: {}", game);
                        return Err(SquadOvError::BadRequest);
                    }
                }
                Ok(())
            } else {
                log::error!("Invalid game partition ID format: {}", &data.object.key);
                Err(SquadOvError::BadRequest)
            }
        } else {
            log::error!("Combat Log S3 Key in the incorrect format: {}", &data.object.key);
            Err(SquadOvError::BadRequest)
        }
    }
}

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(long)]
    bucket: Option<String>,
    #[structopt(long)]
    object: Option<String>,
    #[structopt(long)]
    creds: Option<String>,
    #[structopt(long)]
    profile: Option<String>,
    #[structopt(long)]
    username: Option<String>,
    #[structopt(long)]
    password: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,squadov_common=info,combat_log_report_generator=info");
    env_logger::init();

    let aws_region = std::env::var("SQUADOV_AWS_REGION").unwrap();
    let efs_directory = std::env::var("SQUADOV_EFS_DIRECTORY").unwrap();
    let db_secret_id = std::env::var("SQUADOV_LAMBDA_DB_SECRET").unwrap();
    let db_host = std::env::var("SQUADOV_LAMBDA_DBHOST").unwrap();
    let amqp_url = std::env::var("SQUADOV_AMQP_URL").unwrap();
    let es_queue = std::env::var("SQUADOV_ES_RABBITMQ_QUEUE").unwrap();
    log::info!("AWS Region: {}", &aws_region);
    log::info!("EFS Directory: {}", &efs_directory);
    log::info!("Secret ID: {}", &db_secret_id);
    log::info!("DB Host: {}", &db_host);
    log::info!("ES Queue: {}", &es_queue);

    let opts = Options::from_args();

    log::info!("Creating Secret Manager...");
    let secrets_client =  if opts.creds.is_some() && opts.profile.is_some() {
        SecretsManagerClient::new_with(
            HttpClient::new().unwrap(),
            ProfileProvider::with_configuration(&opts.creds.as_ref().unwrap().clone(), &opts.profile.as_ref().unwrap().clone()),
            Region::from_str(&aws_region)?
        )
    } else {
        SecretsManagerClient::new(
            Region::from_str(&aws_region)?
        )
    };

    // Secret string contains a JSON structure of the form:
    // (it technically has more fields but these are the ones we care about)
    #[derive(Deserialize)]
    struct DbSecret {
        username: String,
        password: String,
    }

    let db_creds = if opts.username.is_some() && opts.password.is_some() {
        DbSecret{
            username: opts.username.unwrap(),
            password: opts.password.unwrap(),
        }
    } else if let Some(secret_string) = secrets_client.get_secret_value(GetSecretValueRequest{
        secret_id: db_secret_id,
        ..GetSecretValueRequest::default()
    }).await?.secret_string {
        log::info!("...Found DB Creds.");
        serde_json::from_str::<DbSecret>(&secret_string)?
    } else {
        return Err(SquadOvError::BadRequest);
    };

    let mut conn = PgConnectOptions::new()
        .host(&db_host)
        .username(&db_creds.username)
        .password(&db_creds.password)
        .port(5432)
        .application_name("combat_log_report_generator")
        .database("squadov")
        .statement_cache_capacity(0);
    conn.log_statements(log::LevelFilter::Trace);
    
    let pool = Arc::new(PgPoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .max_lifetime(std::time::Duration::from_secs(60))
        .idle_timeout(std::time::Duration::from_secs(10))
        .connect_with(conn)
        .await?);

    let rmq_config = RabbitMqConfig{
        amqp_url,
        enable_elasticsearch: true,
        elasticsearch_queue: es_queue,
        elasticsearch_workers: 0,
        ..RabbitMqConfig::default()
    };
    let rabbitmq = RabbitMqInterface::new(&rmq_config, Some(pool.clone()), true).await.unwrap();
    let es_itf = Arc::new(ElasticSearchJobInterface::new_producer_only(&rmq_config, rabbitmq.clone(), pool.clone()));

    log::info!("Creating Shared Client...");
    let shared = SharedClient{
        s3: Arc::new(
            if opts.creds.is_some() && opts.profile.is_some() {
                S3Client::new_with(
                    HttpClient::new().unwrap(),
                    ProfileProvider::with_configuration(&opts.creds.as_ref().unwrap().clone(), &opts.profile.as_ref().unwrap().clone()),
                    Region::from_str(&aws_region)?
                )
            } else {
                S3Client::new(Region::from_str(&aws_region)?)
            }
        ),
        efs_directory,
        pool,
        es_itf,
    };

    let shared_ref = &shared;
    if opts.bucket.is_some() && opts.object.is_some() {
        shared_ref.handle_s3_data(S3Record{
            bucket: S3Bucket{
                name: opts.bucket.unwrap(),
            },
            object: S3Object{
                key: opts.object.unwrap(),
            },
        }).await?;
    } else {
        let closure = move |event: Value, _ctx: Context| async move {
            log::info!("Handling S3 Event Notification: {:?}", event);
    
            let payload = serde_json::from_value::<Payload>(event)?;
            for record in payload.records {
                shared_ref.handle_s3_data(record.s3).await?;
            }
    
            Ok::<(), SquadOvError>(())
        };
    
        log::info!("Starting Runtime [Combat Log Report Generator]...");
        lambda_runtime::run(handler_fn(closure)).await?;
    }
    Ok(())
}