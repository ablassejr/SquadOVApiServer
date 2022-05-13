use lambda_runtime::{handler_fn, Context};
use serde::{Serialize, Deserialize};
use serde_json::{Value};
use std::{
    io::Read,
    str::FromStr,
};
use squadov_common::{
    SquadOvError,
    ff14::combatlog::{
        Ff14CombatLogPacket,
    },
    wow::{
        WowCombatLogPacket,
    },
    combatlog::{
        CombatLogPacket,
        db,
        LOG_FLUSH,
    },
    aws::s3,
};
use rusoto_core::Region;
use rusoto_secretsmanager::{
    SecretsManagerClient,
    SecretsManager,
    GetSecretValueRequest,
};
use std::fmt::Debug;
use lru::LruCache;
use async_std::sync::{RwLock, Arc};
use sqlx::{
    ConnectOptions,
    postgres::{PgPool, PgPoolOptions, PgConnectOptions},
};
use chrono::Utc;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use rusoto_s3::{
    S3Client,
};
use uuid::Uuid;

#[derive(Deserialize)]
struct Payload {
    #[serde(rename="Records")]
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct Record {
    kinesis: KinesisData,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
struct KinesisData {
    partition_key: String,
    data: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all="camelCase")]
struct CombatLogData {
    logs: Vec<String>,
}

struct SharedClient {
    s3: Arc<S3Client>,
    combat_log_bucket: String,
    state_cache: Arc<RwLock<LruCache<String, serde_json::Value>>>,
    pool: Arc<PgPool>,
}

const LRU_CACHE_SIZE: usize = 32;

impl SharedClient {
    async fn upload_to_s3_single_form(&self, form: &str, partition: &str, data: Vec<serde_json::Value>) -> Result<(), SquadOvError> {
        // Note that we want to emulate the behavior of Firehose as much as possible.
        // Which means instead of sending a JSON array of all the data, we send each line as a separate JSON object without the beginning and ending brackets [].
        let mut encoder = flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
        for d in data {
            let uncompressed_data = serde_json::to_vec(&d)?;
            encoder.write_all(&uncompressed_data)?;
            encoder.write(b"\n")?;
        }

        let compressed_data = encoder.finish()?;
        let compressed_data_size = compressed_data.len();
        let key = format!(
            "form={form}/partition={partition}/combatlog_{tm}_{id}.gz",
            form=form,
            partition=partition,
            tm=Utc::now().timestamp_millis(),
            id=&Uuid::new_v4(),
        );
        s3::s3_multipart_upload_data(self.s3.clone(), Cursor::new(compressed_data), compressed_data_size, &self.combat_log_bucket, &key).await?;
        Ok(())
    }

    async fn upload_to_s3<TData: Serialize + Debug>(&self, partition: &str, data: Vec<TData>) -> Result<(), SquadOvError> {
        // A previous prototype used AWS Firehose here. However, the problem with using Firehose is that
        // it results in the files in S3 being generated multiple minutes (3-5 minutes from what I saw from my
        // development testing). I think this delay is too much and is pretty much unnecessary. We're guaranteed
        // that this is the only lambda processing events for the combat log in question. Therefore we can just
        // output directly to S3 here instead of waiting for Firehose to batch things up. We'll probably create
        // a few more files in S3 than Firehose would since there's no batching but on the plus side, no latency
        // and having a few extra files doesn't hurt.
        //
        // We do, however, want to replicate what Firehose would have done which is to output the data into
        // different partitions depending on factors such as the partition key and the type of the packet. We also
        // want to make sure the data we write into S3 is compressed.

        // We can assume all the incoming data has the same partition key - we can't assume they all have the same "form."
        // Split the packets into the appropriate forms.
        let mut split_json_data: HashMap<String, Vec<serde_json::Value>> = HashMap::new();
        let json_data: Vec<serde_json::Value> = data.into_iter().map(|x| { Ok(serde_json::to_value(x)?) }).collect::<Result<Vec<serde_json::Value>, SquadOvError>>()?;
        for d in json_data {
            // There's something to be said about making all packets have the same parent structure with a generic type so that
            // we know for sure there's a "Raw", "Parsed", and "Flush" form.
            if let Some(form) = d["data"]["form"].as_str() {
                if let Some(split_data) = split_json_data.get_mut(form) {
                    split_data.push(d);
                } else {
                    split_json_data.insert(form.to_string(), vec![d]);
                }
            }
        }

        if let Some(data) = split_json_data.remove("Raw") {
            log::info!("...Raw: {}", data.len());
            self.upload_to_s3_single_form("Raw", partition, data).await?;
        }

        if let Some(data) = split_json_data.remove("Parsed") {
            log::info!("...Parsed: {}", data.len());
            self.upload_to_s3_single_form("Parsed", partition, data).await?;
        }

        // Flush should be last so we know that Raw/Parsed got written out successfully.
        if let Some(data) = split_json_data.remove("Flush") {
            log::info!("...Flush: {}", data.len());
            self.upload_to_s3_single_form("Flush", partition, data).await?;
        }

        Ok(())
    }

    fn parse_logs<TPacketData: CombatLogPacket>(partition_key: &str, decoded: CombatLogData, cl_state: serde_json::Value) -> Vec<(String, Option<TPacketData::Data>)> {
        decoded.logs.into_iter()
            .map(|x| {
                let result = std::panic::catch_unwind(|| {
                    let parsed = if x == LOG_FLUSH {
                        Ok(Some(TPacketData::create_flush_packet(partition_key.to_string())))
                    } else {
                        TPacketData::parse_from_raw(partition_key.to_string(), x.clone(), cl_state.clone())
                    };
                    (x.clone(), parsed)
                });

                // Note that result is of type Result<(String, Result<Option<TPacketData>, SquadOverror>), Err>
                // We want to boil this down to just the inner type.
                match result {
                    Ok(y) => y,
                    Err(e) => (
                        x,
                        Err(
                            SquadOvError::InternalError(
                                format!("Parse Panic: {:?}", e)
                            ),
                        )
                    ),
                }
            })
            .filter(|x| {
                if let Err(err) = &x.1 {
                    log::warn!("Failed to parse Combat Log Line: {:?} - {}", err, &x.0);
                }
                x.1.is_ok()
            })
            .map(|x| {
                (x.0, x.1.unwrap())
            })
            .collect()
    }

    fn split_raw_parsed<TPacketData: CombatLogPacket>(partition_id: &str, data: Vec<(String, Option<TPacketData::Data>)>) -> (Vec<TPacketData::Data>, Vec<TPacketData::Data>) {
        let raw_logs = data.iter().map(|x| {
            TPacketData::create_raw_packet(partition_id.to_string(), x.1.as_ref().map(|y| {
                TPacketData::extract_timestamp(y)
            }).unwrap_or(Utc::now()), x.0.clone())
        }).collect::<Vec<TPacketData::Data>>();

        let parsed_logs = data.into_iter().filter(|x| { x.1.is_some() }).map(|x| { x.1.unwrap() }).collect::<Vec<TPacketData::Data>>();
        (raw_logs, parsed_logs)
    }

    async fn generic_parse_combat_log_data<TPacketData: CombatLogPacket>(&self, data: KinesisData) -> Result<(), SquadOvError> {
        // The inner data is base64 encoded - note that we're expecting a JSON structure of FF14 combat logs.
        // The data that we get is BASE64(GZIP(JSON)) so we need to reverse those operations to
        // properly decode the packet.
        let decoded = serde_json::from_slice::<CombatLogData>(&{
            let mut uncompressed_data: Vec<u8> = Vec::new();
            {
                let raw_data = base64::decode(&data.data)?;
                let mut decoder = flate2::read::GzDecoder::new(&*raw_data);
                decoder.read_to_end(&mut uncompressed_data)?;
            }
            uncompressed_data
        })?;

        // Get combat log state - ideally grab it from our cache.
        let cl_state = {
            if let Some(cl_state) = {
                let mut state_cache = self.state_cache.write().await;
                state_cache.get(&data.partition_key).cloned()
            } {
                cl_state
            } else {
                let cl_state = db::get_combat_log_state(&*self.pool, &data.partition_key).await?;

                let mut state_cache = self.state_cache.write().await;
                state_cache.put(data.partition_key.clone(), cl_state.clone());

                cl_state
            }
        };

        // We do a best effort parsing of all the combat log lines. If any one line fails to parse,
        // that doesn't prevent the entire batch from being parsed. We ignore that line and move on.
        log::info!("Parse Logs...");
        let parsed_logs = Self::parse_logs::<TPacketData>(&data.partition_key, decoded, cl_state);

        log::info!("Split Logs...");
        let (raw_logs, parsed_logs) = Self::split_raw_parsed::<TPacketData>(&data.partition_key, parsed_logs);
        
        // Stream the raw and parsed data into AWS s3.
        // Note that to process this data we will rely on an S3 event notification to determine
        // when the flushed object is written.
        log::info!("Upload raw {}...", raw_logs.len());
        self.upload_to_s3::<TPacketData::Data>(&data.partition_key, raw_logs).await?;

        log::info!("Upload parsed {}...", parsed_logs.len());
        self.upload_to_s3::<TPacketData::Data>(&data.partition_key, parsed_logs).await?;

        log::info!("...Finish!");
        Ok(())
    }

    async fn handle_kinesis_data(&self, data: KinesisData) -> Result<(), SquadOvError> {
        // Note that our partition keys will be of the form GAME_UUID. The UUID can be a match UUID
        // or a view UUID depending on the game.
        if data.partition_key.starts_with("ff14_") {
            self.generic_parse_combat_log_data::<Ff14CombatLogPacket>(data).await?;
        } else if data.partition_key.starts_with("wow_") {
            self.generic_parse_combat_log_data::<WowCombatLogPacket>(data).await?;
        } else {
            log::warn!("...Invalid Game Partition Key: {}", &data.partition_key);
            return Err(SquadOvError::BadRequest);
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,squadov_common=info,combat_log_parser=info");
    env_logger::init();

    // Pull environment variables.
    let aws_region = std::env::var("SQUADOV_AWS_REGION").unwrap();
    let secret_id = std::env::var("SQUADOV_LAMBDA_DB_SECRET").unwrap();
    let db_host = std::env::var("SQUADOV_LAMBDA_DBHOST").unwrap();
    let combat_log_bucket = std::env::var("SQUADOV_COMBAT_LOG_BUCKET").unwrap();
    log::info!("AWS Region: {}", &aws_region);
    log::info!("Secret ID: {}", &secret_id);
    log::info!("DB Host: {}", &db_host);
    log::info!("CL Bucket: {}", &combat_log_bucket);

    log::info!("Creating Secret Manager...");
    let secrets_client = SecretsManagerClient::new(
        Region::from_str(&aws_region)?
    );

    log::info!("Getting DB Secret...");
    let secret_object = secrets_client.get_secret_value(GetSecretValueRequest{
        secret_id,
        ..GetSecretValueRequest::default()
    }).await?;

    // Secret string contains a JSON structure of the form:
    // (it technically has more fields but these are the ones we care about)
    #[derive(Deserialize)]
    struct DbSecret {
        username: String,
        password: String,
    }

    let creds = if let Some(secret_string) = secret_object.secret_string {
        log::info!("...Found Creds.");
        serde_json::from_str::<DbSecret>(&secret_string)?
    } else {
        return Err(SquadOvError::BadRequest);
    };

    let mut conn = PgConnectOptions::new()
        .host(&db_host)
        .username(&creds.username)
        .password(&creds.password)
        .port(5432)
        .application_name("combat_log_report_generator")
        .database("squadov")
        .statement_cache_capacity(0);
    conn.log_statements(log::LevelFilter::Trace);

    log::info!("Creating Shared Client...");
    let shared = SharedClient{
        s3: Arc::new(S3Client::new(
            Region::from_str(&aws_region)?
        )),
        combat_log_bucket,
        pool: Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .max_lifetime(std::time::Duration::from_secs(60))
            .idle_timeout(std::time::Duration::from_secs(10))
            .connect_with(conn)
            .await?
        ),
        state_cache: Arc::new(RwLock::new(LruCache::new(LRU_CACHE_SIZE))),
    };

    let shared_ref = &shared;
    let closure = move |event: Value, ctx: Context| async move {
        log::info!("Handling Kinesis Record from {:?}", ctx);
        
        // A note about ordering guarantees. In Kinesis, we're guaranteed that the data in a data stream
        // is processed in order. And FURTHERMORE, if we use the parallization factor parameter (which we do),
        // we're also further guaranteed that messages with the same partition key will get batched and sent
        // to the same lambda function invocation with their ordering preserved. Therefore, we do not have to
        // worry that another lambda is concurrently processing the same combat log.
        // 
        // See: https://aws.amazon.com/blogs/compute/new-aws-lambda-scaling-controls-for-kinesis-and-dynamodb-event-sources/
        let payload = serde_json::from_value::<Payload>(event)?;
        for record in payload.records {
            shared_ref.handle_kinesis_data(record.kinesis).await?;
        }

        Ok::<(), SquadOvError>(())
    };

    log::info!("Starting Runtime [Combat Log Parser]...");
    lambda_runtime::run(handler_fn(closure)).await?;
    Ok(())
}