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
#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "error,squadov_api_server=debug,actix_web=debug,actix_http=debug");
    std::env::set_var("SQLX_LOG", "0");
    env_logger::init();

    let opts = Options::from_args();
    let raw_cfg = fs::read_to_string(opts.config)?;
    let config : api::ApiConfig = toml::from_str(&raw_cfg).unwrap();
    let config2 = config.clone();

    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);
    let app = Arc::new(api::ApiApplication::new(&config).await);
        
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