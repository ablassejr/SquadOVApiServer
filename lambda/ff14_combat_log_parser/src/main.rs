use lambda_runtime::{handler_fn, Context, Error};
use serde::Deserialize;
use serde_json::{Value};
use sqlx::postgres::{PgPool, PgPoolOptions};
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

    // Get database secret AWS secret manager. This should have
    // already been created for the RDS proxy so it should exist.
    let secrets_client = SecretsManagerClient::new(
        Region::from_str(&std::env::var("SQUADOV_LAMBDA_REGION")?)?
    );

    let secret_object = secrets_client.get_secret_value(GetSecretValueRequest{
        secret_id: std::env::var("SQUADOV_LAMBDA_DB_SECRET")?,
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
        serde_json::from_str::<DbSecret>(&secret_string)?
    } else {
        return Err(SquadOvError::BadRequest);
    };

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
            // URL format: 
            // postgresql://USERNAME:PASSWORD@HOSTNAME:PORT/DBNAME
            // We can hardcode port and dbname.
            // Hostname and username should come via environment variables to make it configurable.
            // Password comes from the IAM token that we got above.
            .connect(&format!(
                "postgresql://{user}:{pass}@{host}:5432/squadov",
                user=&creds.username,
                pass=&creds.password,
                host=std::env::var("SQUADOV_LAMBDA_DBHOST").unwrap(),
            ))
            .await?),
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

    lambda_runtime::run(handler_fn(closure)).await?;
    Ok(())
}