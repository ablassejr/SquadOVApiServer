#[macro_use]
extern crate log;

mod api;

use tokio;
use actix_rt;
use actix_web::{App, HttpServer, web};
use actix_web::middleware::{Logger, Compress};
use api::api_service;
use structopt::StructOpt;
use actix_cors::{Cors};
use std::fs;
use std::sync::Arc;
use squadov_common::TaskWrapper;

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
    std::env::set_var("RUST_LOG", "info,squadov_api_server=debug,actix_web=debug,actix_http=debug");
    std::env::set_var("SQLX_LOG", "0");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config)?;
    let config : api::ApiConfig = toml::from_str(&raw_cfg).unwrap();
    let app = Arc::new(api::ApiApplication::new(&config).await);

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
        } else {
            log::error!("Invalid mode: {}", &mode);
        }
        Ok(())
    } else {
        let local = tokio::task::LocalSet::new();
        let sys = actix_rt::System::run_in_tokio("server", &local);
        let config2 = config.clone();
        
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
                        .allowed_methods(vec!["GET", "POST", "OPTIONS"])
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