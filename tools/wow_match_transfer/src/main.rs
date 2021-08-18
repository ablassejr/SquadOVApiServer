use squadov_common::{
    SquadOvError,
};
use structopt::StructOpt;
use std::sync::Arc;
use sqlx::{
    postgres::{
        PgPoolOptions,
        types::PgRange,
    },
    Row,
};
use uuid::Uuid;
use chrono::{DateTime,Utc};
use std::ops::{Bound, Bound::{
    Excluded,
    Included
}};

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    src: String,
    #[structopt(short, long)]
    dest: String,
}

fn get_bound(bnd: &Bound<DateTime<Utc>>) -> DateTime<Utc> {
    if let Excluded(x) = bnd {
        x.clone()
    } else if let Included(x) = bnd {
        x.clone()
    } else {
        Utc::now()
    }
}

#[tokio::main]
async fn main() -> Result<(), SquadOvError> {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("RUST_LOG", "info,wow_match_transfer=debug");
    std::env::set_var("SQLX_LOG", "0");
    env_logger::init();

    let opts = Options::from_args();

    let src_pool = Arc::new(PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .max_lifetime(std::time::Duration::from_secs(6*60*60))
        .idle_timeout(std::time::Duration::from_secs(3*60*60))
        .connect(&opts.src)
        .await
        .unwrap());

    let dst_pool = Arc::new(PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .max_lifetime(std::time::Duration::from_secs(6*60*60))
        .idle_timeout(std::time::Duration::from_secs(3*60*60))
        .connect(&opts.dest)
        .await
        .unwrap());

    log::info!("Transfer WoW Arena Matches.");
    let mut count = 0;
    loop {
        log::info!("...{}", count);
        let tasks = sqlx::query(
            "
            SELECT *
            FROM squadov.new_wow_arenas
            WHERE transferred = FALSE
            LIMIT 1000
            ",
        )
            .fetch_all(&*src_pool)
            .await.unwrap();

        if tasks.is_empty() {
            break;
        }

        let match_ids: Vec<Uuid> = tasks.iter().map(|x| {
            x.get::<Uuid, &str>("match_uuid")
        }).collect();
        let mut tx = dst_pool.begin().await.unwrap();
        for t in &tasks {
            let rng = t.get::<PgRange<DateTime<Utc>>, &str>("tr");
            sqlx::query!(
                "
                INSERT INTO squadov.new_wow_arenas (
                    match_uuid,
                    tr,
                    combatants_key,
                    instance_id,
                    arena_type
                )
                VALUES (
                    $1,
                    tstzrange($2::TIMESTAMPTZ, $3::TIMESTAMPTZ, '[]'),
                    $4,
                    $5,
                    $6
                )
                ",
                t.get::<Uuid, &str>("match_uuid"),
                get_bound(&rng.start),
                get_bound(&rng.end),
                t.get::<String, &str>("combatants_key"),
                t.get::<i32, &str>("instance_id"),
                t.get::<String, &str>("arena_type"),
            )
                .execute(&mut tx)
                .await.unwrap();
        }
        tx.commit().await.unwrap();

        let mut tx = src_pool.begin().await.unwrap();
        sqlx::query(
            "
            UPDATE squadov.new_wow_arenas
            SET transferred = TRUE
            WHERE match_uuid = ANY($1)
            "
        )
            .bind(&match_ids)
            .execute(&mut tx)
            .await.unwrap();
        tx.commit().await.unwrap();
        count += tasks.len() as i64;
    }

    log::info!("Transfer WoW Encounter Matches.");
    let mut count = 0;
    loop {
        log::info!("...{}", count);
        let tasks = sqlx::query(
            "
            SELECT *
            FROM squadov.new_wow_encounters
            WHERE transferred = FALSE
            LIMIT 1000
            ",
        )
            .fetch_all(&*src_pool)
            .await.unwrap();

        if tasks.is_empty() {
            break;
        }

        let match_ids: Vec<Uuid> = tasks.iter().map(|x| {
            x.get::<Uuid, &str>("match_uuid")
        }).collect();
        let mut tx = dst_pool.begin().await.unwrap();
        for t in &tasks {
            let rng = t.get::<PgRange<DateTime<Utc>>, &str>("tr");
            sqlx::query!(
                "
                INSERT INTO squadov.new_wow_encounters (
                    match_uuid,
                    tr,
                    combatants_key,
                    encounter_id,
                    difficulty,
                    instance_id
                )
                VALUES (
                    $1,
                    tstzrange($2::TIMESTAMPTZ, $3::TIMESTAMPTZ, '[]'),
                    $4,
                    $5,
                    $6,
                    $7
                )
                ",
                t.get::<Uuid, &str>("match_uuid"),
                get_bound(&rng.start),
                get_bound(&rng.end),
                t.get::<String, &str>("combatants_key"),
                t.get::<i32, &str>("encounter_id"),
                t.get::<i32, &str>("difficulty"),
                t.get::<i32, &str>("instance_id"),
            )
                .execute(&mut tx)
                .await.unwrap();
        }
        tx.commit().await.unwrap();

        let mut tx = src_pool.begin().await.unwrap();
        sqlx::query(
            "
            UPDATE squadov.new_wow_encounters
            SET transferred = TRUE
            WHERE match_uuid = ANY($1)
            "
        )
            .bind(&match_ids)
            .execute(&mut tx)
            .await.unwrap();
        tx.commit().await.unwrap();
        count += tasks.len() as i64;
    }

    log::info!("Finish migration.");
    Ok(())
}