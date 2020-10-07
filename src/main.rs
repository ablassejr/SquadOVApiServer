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

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    config: std::path::PathBuf,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "squadov_api_server=info,actix_web=info");
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
            .app_data(app.clone())
            .wrap(Logger::default())
            .service(api_service::create_service())
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await?;
    sys.await?;
    Ok(res)
}