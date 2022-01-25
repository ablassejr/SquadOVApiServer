use lambda_runtime::{handler_fn, Context, Error};
use serde::Deserialize;
use serde_json::{Value};
use sqlx::postgres::{PgPool, PgPoolOptions, PgConnectOptions};
use std::sync::Arc;
use rusoto_core::{Region};
use rusoto_secretsmanager::{
    SecretsManagerClient,
    SecretsManager,
    GetSecretValueRequest,
};
use std::str::FromStr;
use squadov_common::SquadOvError;

struct SharedClient {
    pool: Arc<PgPool>
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,ff14_combat_log_parser=info");
    std::env::set_var("SQLX_LOG", "0");
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
    let shared = SharedClient{
        pool: Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(1)
            .max_lifetime(std::time::Duration::from_secs(6*60*60))
            .idle_timeout(std::time::Duration::from_secs(3*60*60))
            .connect_with(PgConnectOptions::new()
                .host(&db_host)
                .username(&creds.username)
                .password(&creds.password)
                .port(5432)
                .application_name("ff14_combat_log_parser")
                .database("squadov")
                .statement_cache_capacity(0)
            )
            .await?
        ),
    };

    let shared_ref = &shared;
    let closure = move |event: Value, ctx: Context| async move {
        log::info!("Handling Kinesis Record: {:?}", event.as_str());
        log::info!("Do SQL Query: {}",
            sqlx::query!(
                r#"
                SELECT 1 AS "test!" 
                "#
            )
                .fetch_one(&*shared_ref.pool)
                .await?
                .test
        );
        log::info!("Execution Context: {:?}", &ctx);
        Ok::<(), Error>(())
    };

    log::info!("Starting Runtime...");
    lambda_runtime::run(handler_fn(closure)).await?;
    Ok(())
}