use squadov_common::{
    SquadOvError,
};
use structopt::StructOpt;
use std::sync::Arc;
use async_std::sync::RwLock;
use sqlx::{
    postgres::{
        PgPoolOptions
    },
};
use uuid::Uuid;

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    db: String,
    #[structopt(short, long)]
    connections: u32,
}

#[derive(Debug, Clone)]
struct WowMatchTask {
    uuid: Uuid,
    typ: i32
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,wow_db_migrater=debug");
    std::env::set_var("SQLX_LOG", "0");
    env_logger::init();

    let opts = Options::from_args();

    let pool = Arc::new(PgPoolOptions::new()
        .min_connections(1)
        .max_connections(opts.connections)
        .max_lifetime(std::time::Duration::from_secs(6*60*60))
        .idle_timeout(std::time::Duration::from_secs(3*60*60))
        .connect(&opts.db)
        .await
        .unwrap());

    let tasks = Arc::new(RwLock::new(sqlx::query_as!(
        WowMatchTask,
        r#"
        SELECT match_uuid AS "uuid!", 0 AS "typ!"
        FROM squadov.wow_arenas
        UNION
        SELECT match_uuid AS "uuid!", 1 AS "typ!"
        FROM squadov.wow_challenges
        UNION
        SELECT match_uuid AS "uuid!", 2 AS "typ!"
        FROM squadov.wow_encounters
        "#
    )
        .fetch_all(&*pool)
        .await?));

    let handles: Vec<_> = (0..opts.connections).into_iter().map(|_x| {
        let tpool = pool.clone();
        let ttasks = tasks.clone();
        tokio::task::spawn(async move {
            loop {
                let next_task = {
                    let mut t = ttasks.write().await;
                    t.pop()
                };

                if next_task.is_none() {
                    break;
                }

                let next_task = next_task.unwrap();
                if next_task.typ == 0 {
                    let _ = sqlx::query!(
                        "
                        SELECT * FROM transfer_wow_arenas($1)
                        ",
                        &next_task.uuid
                    )
                        .execute(&*tpool)
                        .await;
                } else if next_task.typ == 1 {
                    let _ = sqlx::query!(
                        "
                        SELECT * FROM transfer_wow_challenges($1)
                        ",
                        &next_task.uuid
                    )
                        .execute(&*tpool)
                        .await;
                } else if next_task.typ == 2 {
                    let _ = sqlx::query!(
                        "
                        SELECT * FROM transfer_wow_encounters($1)
                        ",
                        &next_task.uuid
                    )
                        .execute(&*tpool)
                        .await;
                }
            }
        })
    }).collect();

    for hnd in handles {
        let _ = hnd.await;
    }

    Ok(())
}