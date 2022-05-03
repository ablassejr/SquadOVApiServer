#[macro_use]
extern crate log;

mod api;

use structopt::StructOpt;
use std::{fs, sync::Arc};
use uuid::Uuid;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
    #[structopt(short, long)]
    db: u32,
    #[structopt(short, long)]
    workers: usize,
}

pub fn start_cleanup_loop(app: Arc<api::ApiApplication>) {
    tokio::task::spawn(async move {
        loop {
            log::info!("Doing cleanup loop...");

            let old_unpublished_clips: Vec<Uuid> = sqlx::query!(
                "
                SELECT clip_uuid
                FROM squadov.vod_clips
                WHERE NOT published
                    AND tm < (NOW() - INTERVAL '1 day')
                "
            )
                .fetch_all(&*app.pool)
                .await
                .unwrap_or(vec![])
                .into_iter()
                .map(|x| {
                    x.clip_uuid
                })
                .collect();
            log::info!("Found {} Unpublished Clips", old_unpublished_clips.len());
            for x in old_unpublished_clips {
                match app.vod_itf.request_delete_vod(&x).await {
                    Ok(_) => (),
                    Err(err) => log::warn!("...Failed to delete VOD [{}] {:?}", &x, err),
                }
            }

            // Doing this once per day should be sufficient...
            tokio::time::sleep(tokio::time::Duration::from_secs(86400)).await;
        }
    });
}

fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,singleton_event_processing_worker=debug,actix_web=debug,actix_http=debug,librdkafka=info,rdkafka::client=info,sqlx=info");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config.clone()).unwrap();
    let mut config : api::ApiConfig = toml::from_str(&raw_cfg).unwrap();
    config.database.connections = opts.db;
    config.database.heavy_connections = opts.db;
    config.rabbitmq.enable_rso = true;
    config.rabbitmq.enable_lol = true;
    config.rabbitmq.enable_tft = true;
    config.rabbitmq.enable_valorant = true;
    config.rabbitmq.enable_vod = false;
    config.rabbitmq.enable_csgo = false;
    config.rabbitmq.enable_steam = true;
    config.rabbitmq.enable_twitch = true;
    config.rabbitmq.enable_sharing = false;
    config.rabbitmq.enable_elasticsearch = false;
    config.rabbitmq.prefetch_count = 2;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(opts.workers)
        .build()
        .unwrap()
        .block_on(async move {
            // Only use the provided config to connect to things.
            tokio::task::spawn(async move {
                let app = Arc::new(api::ApiApplication::new(&config, "singleton_event").await);
                api::start_event_loop(app.clone());
                start_cleanup_loop(app.clone());

                loop {
                    async_std::task::sleep(std::time::Duration::from_secs(1)).await;
                }
            }).await.unwrap();
            Ok(())
        })
}