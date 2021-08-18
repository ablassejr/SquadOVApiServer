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
            ORDER BY tr ASC
            LIMIT 1000
            ",
        )
            .fetch_all(&*src_pool)
            .await.unwrap();
        log::info!("\tpost select.");

        if tasks.is_empty() {
            break;
        }

        let mut tx = dst_pool.begin().await.unwrap();

        let mut task_sql: Vec<String> = Vec::new();
        task_sql.push(String::from("
            INSERT INTO squadov.new_wow_arenas (
                match_uuid,
                tr,
                combatants_key,
                instance_id,
                arena_type
            )
            VALUES 
        "));

        for t in &tasks {
            let rng = t.get::<PgRange<DateTime<Utc>>, &str>("tr");
            task_sql.push(format!("(
                {match_uuid},
                tstzrange({start}::TIMESTAMPTZ, {end}::TIMESTAMPTZ, '[]'),
                {combatants_key},
                {instance_id},
                {arena_type}
            )",
                match_uuid=squadov_common::sql_format_string(&t.get::<Uuid, &str>("match_uuid").to_string()),
                start=squadov_common::sql_format_time(&get_bound(&rng.start)),
                end=squadov_common::sql_format_time(&get_bound(&rng.end)),
                combatants_key=squadov_common::sql_format_string(&t.get::<String, &str>("combatants_key")),
                instance_id=t.get::<i32, &str>("instance_id"),
                arena_type=squadov_common::sql_format_string(&t.get::<String, &str>("arena_type")),
            ));
            task_sql.push(String::from(","));
        }

        task_sql.truncate(task_sql.len() - 1);
        task_sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&task_sql.join("")).execute(&mut tx).await?;
        tx.commit().await.unwrap();
        log::info!("\tpost insert.");

        let mut tx = src_pool.begin().await.unwrap();
        sqlx::query(
            "
            UPDATE squadov.new_wow_arenas AS wa
            SET transferred = TRUE
            FROM (
                SELECT match_uuid
                FROM squadov.new_wow_arenas
                WHERE transferred = FALSE
                ORDER BY tr ASC
                LIMIT 1000
            ) AS sub
            WHERE wa.match_uuid=sub.match_uuid
            "
        )
            .execute(&mut tx)
            .await.unwrap();
        tx.commit().await.unwrap();
        log::info!("\tpost update.");
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
            ORDER BY tr ASC
            LIMIT 1000
            ",
        )
            .fetch_all(&*src_pool)
            .await.unwrap();
        log::info!("\tpost select.");

        if tasks.is_empty() {
            break;
        }

        let mut tx = dst_pool.begin().await.unwrap();

        let mut task_sql: Vec<String> = Vec::new();
        task_sql.push(String::from("
            INSERT INTO squadov.new_wow_encounters (
                match_uuid,
                tr,
                combatants_key,
                encounter_id,
                difficulty,
                instance_id
            )
            VALUES
        "));

        for t in &tasks {
            let rng = t.get::<PgRange<DateTime<Utc>>, &str>("tr");
            task_sql.push(format!("(
                {match_uuid},
                tstzrange({start}::TIMESTAMPTZ, {end}::TIMESTAMPTZ, '[]'),
                {combatants_key},
                {encounter_id},
                {difficulty},
                {instance_id}
            )",
                match_uuid=squadov_common::sql_format_string(&t.get::<Uuid, &str>("match_uuid").to_string()),
                start=squadov_common::sql_format_time(&get_bound(&rng.start)),
                end=squadov_common::sql_format_time(&get_bound(&rng.end)),
                combatants_key=squadov_common::sql_format_string(&t.get::<String, &str>("combatants_key")),
                encounter_id=t.get::<i32, &str>("encounter_id"),
                difficulty=t.get::<i32, &str>("difficulty"),
                instance_id=t.get::<i32, &str>("instance_id"),
            ));
            task_sql.push(String::from(","));
        }

        task_sql.truncate(task_sql.len() - 1);
        task_sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&task_sql.join("")).execute(&mut tx).await?;

        tx.commit().await.unwrap();
        log::info!("\tpost insert.");

        let mut tx = src_pool.begin().await.unwrap();
        sqlx::query(
            "
            UPDATE squadov.new_wow_encounters AS we
            SET transferred = TRUE
            FROM (
                SELECT match_uuid
                FROM squadov.new_wow_encounters
                WHERE transferred = FALSE
                ORDER BY tr ASC
                LIMIT 1000
            ) AS sub
            WHERE we.match_uuid=sub.match_uuid
            "
        )
            .execute(&mut tx)
            .await.unwrap();
        tx.commit().await.unwrap();
        log::info!("\tpost update.");
        count += tasks.len() as i64;
    }

    log::info!("Finish migration.");
    Ok(())
}