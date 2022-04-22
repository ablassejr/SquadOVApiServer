use config::{Config, Environment, File};
use serde::Deserialize;
use squadov_common::{
    SquadOvError,
    rabbitmq::{RabbitMqInterface, RabbitMqConfig},
    elastic::{
        ElasticSearchConfig,
        ElasticSearchClient,
        rabbitmq::ElasticSearchJobInterface,
    },
};
use std::sync::Arc;
use sqlx::{
    ConnectOptions,
    postgres::{
        PgPoolOptions,
        PgConnectOptions,
    },
};
use uuid::Uuid;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    manual: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct SyncConfig {
    db_endpoint: String,
    db_username: String,
    db_password: String,
    db_connections: u32,
    rabbitmq: RabbitMqConfig,
    elasticsearch: ElasticSearchConfig,
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,elasticsearch_sync=debug,actix_web=debug,actix_http=debug");
    env_logger::init();

    // Initialize shared state to access the database as well as any other shared configuration.
    let opts = Options::from_args();
    let config: SyncConfig = Config::builder()
        .add_source(File::with_name("config/elasticsearch_sync.toml"))
        .add_source(Environment::with_prefix("squadov").separator("__").prefix_separator("_").try_parsing(true))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();

    tokio::task::spawn(async move {
        let mut conn = PgConnectOptions::new()
            .host(&config.db_endpoint)
            .username(&config.db_username)
            .password(&config.db_password)
            .port(5432)
            .application_name("squadov_es_sync")
            .database("squadov")
            .statement_cache_capacity(0);
        conn.log_statements(log::LevelFilter::Trace);
        let pool = Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(config.db_connections)
            .max_lifetime(std::time::Duration::from_secs(60))
            .idle_timeout(std::time::Duration::from_secs(20))
            .connect_with(conn)
            .await
            .unwrap());
        

        let rabbitmq = RabbitMqInterface::new(&config.rabbitmq, Some(pool.clone()), true).await.unwrap();
        let es_api = Arc::new(ElasticSearchClient::new(config.elasticsearch.clone()));
        let es_itf = Arc::new(ElasticSearchJobInterface::new(es_api.clone(), &config.elasticsearch, &config.rabbitmq, rabbitmq.clone(), pool.clone()));

        if let Some(manual) = opts.manual {
            es_itf.handle_sync_vod(&[manual.clone()]).await.unwrap();
        } else {
            for _i in 0..config.rabbitmq.elasticsearch_workers {
                RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.elasticsearch_queue.clone(), es_itf.clone(), config.rabbitmq.prefetch_count).await.unwrap();
            }

            loop {
                async_std::task::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    }).await.unwrap();
    Ok(())
}