use squadov_common::{
    SquadOvError,
    rabbitmq::{RabbitMqInterface, RabbitMqConfig},
    WowCombatLogRmqInterface,
};
use structopt::StructOpt;
use std::fs;
use std::sync::Arc;
use serde::Deserialize;
use sqlx::{
    postgres::{
        PgPoolOptions
    },
};

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: String,
}

#[derive(Deserialize,Debug,Clone)]
struct Config {
    db: String,
    threads: u32,
    rabbitmq: RabbitMqConfig,
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,wow_combatlog_handler=debug");
    std::env::set_var("SQLX_LOG", "0");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config).unwrap();
    let config : Config = toml::from_str(&raw_cfg).unwrap();

    tokio::task::spawn(async move {
        let pool = Arc::new(PgPoolOptions::new()
            .min_connections(1)
            .max_connections(config.threads)
            .max_lifetime(std::time::Duration::from_secs(6*60*60))
            .idle_timeout(std::time::Duration::from_secs(3*60*60))
            .connect(&config.db)
            .await
            .unwrap());

        let rabbitmq = RabbitMqInterface::new(&config.rabbitmq, pool.clone(), true).await.unwrap();

        for _i in 0..config.threads {
            let itf = Arc::new(WowCombatLogRmqInterface::new(&config.rabbitmq, rabbitmq.clone(), pool.clone()));
            RabbitMqInterface::add_listener(rabbitmq.clone(), config.rabbitmq.wow_combatlog_queue.clone(), itf.clone(), config.rabbitmq.prefetch_count).await.unwrap();
        }

        log::info!("Start Listening for WoW Combat Log Payloads...");
        loop {
            async_std::task::sleep(std::time::Duration::from_secs(10)).await;
        }
    }).await.unwrap();

    Ok(())
}