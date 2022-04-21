#[macro_use]
extern crate log;

mod api;

use structopt::StructOpt;
use std::{fs, sync::Arc};

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
    #[structopt(short, long)]
    db: u32,
    #[structopt(short, long)]
    workers: usize,
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

                loop {
                    log::info!("Checking for Backfill ES Vods.");
                    // A bit hacky but use this opportunity to do backfill for all the VODs and clips that need to be synced to ES.
                    if let Ok(backfill_es_vods) = sqlx::query!(
                        "
                        SELECT v.video_uuid
                        FROM squadov.vods AS v
                        INNER JOIN squadov.matches AS m
                            ON m.uuid = v.match_uuid
                        WHERE request_sync_elasticsearch IS NULL
                            AND m.game IS NOT NULL
                        ORDER BY v.end_time DESC
                        LIMIT 10000
                        "
                    )
                        .fetch_all(&*app.pool)
                        .await
                    {
                        log::info!("...Backfilling {} VODs to ES.", backfill_es_vods.len());
                        for (idx, v) in backfill_es_vods.chunks(10).enumerate() {
                            app.es_itf.request_sync_vod(v.iter().map(|x| { x.video_uuid.clone() }).collect()).await.unwrap();

                            if idx % 5 == 0 {
                                async_std::task::sleep(std::time::Duration::from_millis(1)).await;
                            }
                        }
                    }

                    log::info!("...Finish Backfill - Sleep for next.");
                    async_std::task::sleep(std::time::Duration::from_millis(10)).await;
                }
            }).await.unwrap();
            Ok(())
        })
}