#[macro_use]
extern crate log;

mod api;

use squadov_common::{
    SquadOvError,
    rabbitmq::{
        RabbitMqInterface,
        RabbitMqListener,
    },
};
use structopt::StructOpt;
use std::{fs, sync::Arc};
use uuid::Uuid;
use async_trait::async_trait;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
    #[structopt(short, long)]
    workers: usize,
    #[structopt(short, long)]
    threads: usize,
}

pub struct WowTaskHandler {
    app: Arc<api::ApiApplication>,
}

impl WowTaskHandler {
    pub fn new (app: Arc<api::ApiApplication>) -> Self {
        Self {
            app,
        }
    }
}

impl WowTaskHandler {
    async fn handle_transfer_reports(&self, view_id: &Uuid) -> Result<(), SquadOvError>{
        // We need to do the same thing as the combat log report generator except that instead of creating them from
        // the parsed combat log, we need to create them from the existing data.

        // Character Reports
        // - Combatants
        // - Characters
        // - Loadouts

        // Event Reports
        // - Deaths
        // - Auras
        // - Encounters
        // - Resurrections
        // - Aura Breaks
        // - Spell Casts
        // - Death Recaps
        
        // Stat Reports
        // - DPS (per char)
        // - HPS (per char)
        // - DRPS (per char)
        // - Summary
        Ok(())
    }
}

#[async_trait]
impl RabbitMqListener for WowTaskHandler {
    async fn handle(&self, data: &[u8], _queue: &str) -> Result<(), SquadOvError> {
        let view_ids: Vec<Uuid> = serde_json::from_slice(data)?;
        log::info!("Handle Combat Log Transfer RabbitMQ Task: {:?}", &view_ids);
        for view_id in view_ids {
            if let Err(err) = self.handle_transfer_reports(&view_id).await {
                log::error!("Failed to transfer report: {:?}", err);
            }
        }

        Ok(())
    }
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,wow_combat_log_transfer_worker=debug,actix_web=debug,actix_http=debug,sqlx=info");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config.clone()).unwrap();
    let mut config : api::ApiConfig = toml::from_str(&raw_cfg).unwrap();
    config.rabbitmq.additional_queues = Some(vec!["wow_combat_log_transfer".to_string()]);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(opts.workers)
        .build()
        .unwrap()
        .block_on(async move {
            // Only use the provided config to connect to things.
            tokio::task::spawn(async move {
                let app = Arc::new(api::ApiApplication::new(&config, "wow_combat_log_transfer_worker").await);
                let handler_itf = Arc::new(WowTaskHandler::new(app.clone()));
                for _i in 0..opts.threads {
                    RabbitMqInterface::add_listener(app.rabbitmq.clone(), "wow_combat_log_transfer".to_string(), handler_itf.clone(), 1).await.unwrap();
                }

                loop {
                    async_std::task::sleep(std::time::Duration::from_secs(1)).await;
                }
            }).await.unwrap();
            Ok(())
        })
}