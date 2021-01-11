#[macro_use]
extern crate log;

mod api;
mod wow_kafka;

use tokio;
use actix_rt;
use actix_web::{App, HttpServer, web};
use actix_web::middleware::{Logger, Compress};
use api::api_service;
use structopt::StructOpt;
use actix_cors::{Cors};
use std::fs;
use rdkafka::config::ClientConfig;
use async_std::sync::{Arc};
use squadov_common::{TaskWrapper};

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
    #[structopt(short, long)]
    mode: Option<String>,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,squadov_api_server=debug,actix_web=debug,actix_http=debug,librdkafka=info,rdkafka::client=info");
    std::env::set_var("SQLX_LOG", "0");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config)?;
    let config : api::ApiConfig = toml::from_str(&raw_cfg).unwrap();
    let app = Arc::new(api::ApiApplication::new(&config).await);

    let mut kafka_config = ClientConfig::new();
    kafka_config.set("bootstrap.servers", &config.kafka.bootstrap_servers);
    kafka_config.set("security.protocol", "SASL_SSL");
    kafka_config.set("sasl.mechanisms", "PLAIN");
    kafka_config.set("sasl.username", &config.kafka.server_keypair.key);
    kafka_config.set("sasl.password", &config.kafka.server_keypair.secret);
    kafka_config.set("enable.auto.offset.store", "false");

    // A hacky way of doing things related to api::ApiApplication...
    if opts.mode.is_some() {
        let mode = opts.mode.unwrap();
        if mode == "vod_fastify" {
            let vods = app.find_vods_without_fastify().await.unwrap();
            for v in vods {
                log::info!("Enqueue job: {}", &v);
                app.vod_fastify_jobs.enqueue(TaskWrapper::new(api::v1::VodFastifyJob{
                    video_uuid: v,
                    app: app.clone(),
                    session_uri: None,
                })).unwrap();
            }
        } else if mode == "wow_manual_parsing" {
            wow_kafka::create_wow_consumer_thread(app.clone(), &kafka_config).await?;
        } else {
            log::error!("Invalid mode: {}", &mode);
        }
        Ok(())
    } else {
        let local = tokio::task::LocalSet::new();
        let sys = actix_rt::System::run_in_tokio("server", &local);
        let config2 = config.clone();

        for _i in 0..config.kafka.wow_combat_log_threads {
            wow_kafka::create_wow_consumer_thread(app.clone(), &kafka_config);
        }
        
        // The API service is primarily used for dealing with API calls.actix_web
        // We're not going to have a web-based interface at the moment (only going to be desktop client-based)
        // so this server doesn't have to serve javascript or the like.
        let res = HttpServer::new(move || {
            App::new()
                .wrap(Compress::default())
                .wrap(
                    Cors::new()
                        .allowed_origin(&config.cors.domain)
                        .allowed_origin("http://127.0.0.1:8080")
                        .allowed_methods(vec!["GET", "POST", "OPTIONS", "DELETE", "PUT"])
                        .allowed_headers(vec![
                            "x-squadov-session-id",
                            "content-type"
                        ])
                        .finish()
                )
                .wrap(Logger::default())
                .app_data(web::Data::new(app.clone()))
                .service(api_service::create_service(config.server.graphql_debug))
            })
            .server_hostname(&config2.server.domain)
            .bind("0.0.0.0:8080")?
            .run()
            .await?;
        sys.await?;
        Ok(res)
    }
}