use lambda_runtime::{handler_fn, Context};
use serde::Deserialize;
use serde_json::{Value};
use sqlx::{
    ConnectOptions,
    postgres::{PgPool, PgPoolOptions, PgConnectOptions},
};
use std::sync::Arc;
use rusoto_core::{Region};
use rusoto_secretsmanager::{
    SecretsManagerClient,
    SecretsManager,
    GetSecretValueRequest,
};
use std::{
    io::Read,
    str::FromStr
};
use squadov_common::SquadOvError;

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
    #[serde(default)]
    generate_reports: bool,
}

struct SharedClient {
    pool: Arc<PgPool>
}

impl SharedClient {
    async fn handle_kinesis_data(&self, data: KinesisData) -> Result<(), SquadOvError> {
        // The inner data is base64 encoded - note that we're expecting a JSON structure of FF14 combat logs.
        log::info!("Handle Kinesis Data: {:?}", data);

        // Ensure that the partition key is for ff14.
        // Note that our partition keys will be of the form GAME_MATCHUUID.
        if !data.partition_key.starts_with("ff14_") {
            log::warn!("...Invalid Game Partition Key: {}", &data.partition_key);
            return Err(SquadOvError::BadRequest);
        }

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
        log::info!("...Decoded {:?}", &decoded);

        // We do a best effort parsing of all the combat log lines. If any one line fails to parse,
        // that doesn't prevent the entire batch from being parsed. We ignore that line and move on.
        for line in decoded.logs {
            log::info!("...Handle Line: {}", &line);
        }

        // Store all the lines in DynamoDB. We chose DynamoDB because it can scale up easily to handle
        // a shit ton of writes and the only thing we'll need to do at the end is to scoop up all the parsed
        // events and create reports.

        // Generate reports. Note that this is a flag sent by the client if this is the last batch of combat
        // log lines for the current partition key.
        if decoded.generate_reports {

        }

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,ff14_combat_log_parser=info,sqlx=info");
    env_logger::init();

    // Pull environment variables.
    let aws_region = std::env::var("SQUADOV_LAMBDA_REGION").unwrap();
    let secret_id = std::env::var("SQUADOV_LAMBDA_DB_SECRET").unwrap();
    let db_host = std::env::var("SQUADOV_LAMBDA_DBHOST").unwrap();
    log::info!("AWS Region: {}", &aws_region);
    log::info!("Secret ID: {}", &secret_id);
    log::info!("DB Host: {}", &db_host);

    // Get database secret AWS secret manager. This should have
    // already been created for the RDS proxy so it should exist.
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

    log::info!("Creating Shared Client...");
    // Create shared state that can be frozen between invocations.
    // Note that for the database, I don't think we will
    // need more than a single connection in the pool since the Lambda
    // will only ever handle one request at a time.
    let mut conn = PgConnectOptions::new()
        .host(&db_host)
        .username(&creds.username)
        .password(&creds.password)
        .port(5432)
        .application_name("ff14_combat_log_parser")
        .database("squadov")
        .statement_cache_capacity(0);
    conn.log_statements(log::LevelFilter::Trace);
    let shared = SharedClient{
        pool: Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .max_lifetime(std::time::Duration::from_secs(6*60*60))
            .idle_timeout(std::time::Duration::from_secs(3*60*60))
            .connect_with(conn)
            .await?
        ),
    };

    let shared_ref = &shared;
    let closure = move |event: Value, _ctx: Context| async move {
        log::info!("Handling Kinesis Record: {:?}", event);
        
        let payload = serde_json::from_value::<Payload>(event)?;
        for record in payload.records {
            shared_ref.handle_kinesis_data(record.kinesis).await?;
        }

        Ok::<(), SquadOvError>(())
    };

    log::info!("Starting Runtime...");
    lambda_runtime::run(handler_fn(closure)).await?;
    Ok(())
}