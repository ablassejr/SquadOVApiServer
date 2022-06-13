mod bot;

use config::{Config, Environment, File};
use serde::Deserialize;
use squadov_common::{
    SquadOvError,
    rabbitmq::{RabbitMqInterface, RabbitMqConfig},
    discord::bot::DiscordBotConfig,
};
use std::sync::Arc;
use sqlx::{
    ConnectOptions,
    postgres::{
        PgPoolOptions,
        PgConnectOptions,
    },
    PgPool,
};
use serenity::{
    prelude::*,
    framework::StandardFramework,
};

#[derive(Deserialize, Clone)]
pub struct BotConfig {
    db_endpoint: String,
    db_username: String,
    db_password: String,
    db_connections: u32,
    rabbitmq: RabbitMqConfig,
    discord: DiscordBotConfig,
    external_workers: u32,
}

pub struct BotClient {
    config: BotConfig,
    discord: Client,
    rabbitmq: Arc<RabbitMqInterface>,
    db: Arc<PgPool>,
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_LOG", "info,discord=debug,actix_web=debug,actix_http=debug,tracing::span=WARN,serenity=WARN");
    env_logger::init();

    let config: BotConfig = Config::builder()
        .add_source(File::with_name("config/discord.toml"))
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
            .application_name("squadov_discord")
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

        let framework = StandardFramework::new();
        let mut bot = BotClient{
            config: config.clone(),
            discord: Client::builder(&config.discord.token, GatewayIntents::default())
                .framework(framework)
                .await
                .unwrap(),
            rabbitmq: RabbitMqInterface::new(&config.rabbitmq, Some(pool.clone()), true).await.unwrap(),
            db: pool,
        };
        bot.start_external_workers().await;

        // start listening for events by starting a single shard
        if let Err(err) = bot.discord.start().await {
            log::info!("Failed to start Discord client: {:?}", err);
        }
    }).await.unwrap();
    Ok(())
}