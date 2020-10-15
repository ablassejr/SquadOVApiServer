#[macro_use]
extern crate log;

mod api;
mod common;

use tokio;
use actix_rt;
use actix_web::{App, HttpServer, web};
use actix_web::middleware::Logger;
use api::api_service;
use structopt::StructOpt;
use actix_cors::{Cors};

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "squadov_api_server=info,actix_web=debug,actix_http=debug");
    env_logger::init();

    let opts = Options::from_args();
    let app = web::Data::new(api::ApiApplication::new(opts.config).await?);

    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);
    // The API service is primarily used for dealing with API calls.actix_web
    // We're not going to have a web-based interface at the moment (only going to be desktop client-based)
    // so this server doesn't have to serve javascript or the like.
    let res = HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::new()
                    .allowed_origin(&app.cors.domain)
                    .allowed_methods(vec!["GET", "POST", "OPTIONS"])
                    .finish()
            )
            .wrap(Logger::default())
            .app_data(app.clone())
            .service(api_service::create_service())
        })
        .bind("0.0.0.0:8080")?
        .run()
        .await?;
    sys.await?;
    Ok(res)
}