use squadov_common::{
    SquadOvError,
    rabbitmq::{
        RabbitMqInterface,
        RabbitMqConfig,
        RabbitMqListener,
    },
};
use structopt::StructOpt;
use std::sync::Arc;
use sqlx::{
    postgres::{
        PgPoolOptions,
        types::PgRange,
        PgPool,
    },
    Row,
};
use uuid::Uuid;
use chrono::{DateTime,Utc};
use std::ops::{Bound, Bound::{
    Excluded,
    Included
}};
use async_trait::async_trait;

pub struct WowTaskHandler {
    src_db: Arc<PgPool>,
    dst_db: Arc<PgPool>,
    encounter: bool,
}

impl WowTaskHandler {
    pub fn new (src_db: Arc<PgPool>, dst_db: Arc<PgPool>, encounter: bool) -> Self {
        Self {
            src_db,
            dst_db,
            encounter,
        }
    }
}

#[async_trait]
impl RabbitMqListener for WowTaskHandler {
    async fn handle(&self, data: &[u8]) -> Result<(), SquadOvError> {
        log::info!("Handle Transfer RabbitMQ Task: {}", if self.encounter { "Encounter" } else { "Arena" });
        let task_ids: Vec<Uuid> = serde_json::from_slice(data)?;
        log::info!("\tselect {}", task_ids.len());
        let tasks = sqlx::query(&format!(
            "
            SELECT *
            FROM squadov.{}
            WHERE match_uuid = ANY($1)
            ",
            if self.encounter {
                "new_wow_encounters"
            } else {
                "new_wow_arenas"
            }))
            .bind(task_ids)
            .fetch_all(&*self.src_db)
            .await.unwrap();
        log::info!("\tpost select.");

        if tasks.is_empty() {
            return Ok(());
        }

        let mut tx = self.dst_db.begin().await.unwrap();

        let mut task_sql: Vec<String> = Vec::new();
        task_sql.push(
            if self.encounter {
                String::from("
                    INSERT INTO squadov.new_wow_encounters (
                        match_uuid,
                        tr,
                        combatants_key,
                        encounter_id,
                        difficulty,
                        instance_id
                    )
                    VALUES
                ")
            } else {
                String::from("
                    INSERT INTO squadov.new_wow_arenas (
                        match_uuid,
                        tr,
                        combatants_key,
                        instance_id,
                        arena_type
                    )
                    VALUES 
                ")
            }
        );

        for t in &tasks {
            let rng = t.get::<PgRange<DateTime<Utc>>, &str>("tr");
            if self.encounter {
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
            } else {
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
            }
            task_sql.push(String::from(","));
        }

        task_sql.truncate(task_sql.len() - 1);
        task_sql.push(String::from(" ON CONFLICT DO NOTHING"));
        sqlx::query(&task_sql.join("")).execute(&mut tx).await?;
        tx.commit().await.unwrap();
        log::info!("\tpost insert.");

        Ok(())
    }
}

#[derive(StructOpt, Debug)]
struct Options {
    #[structopt(short, long)]
    src: String,
    #[structopt(short, long)]
    dest: String,
    #[structopt(short, long)]
    threads: u32,
    #[structopt(short, long)]
    queue: String,
    #[structopt(short, long)]
    encounter: bool,
    #[structopt(short, long)]
    rmq: String,
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

    let rabbitmq = RabbitMqInterface::new(&RabbitMqConfig{
        amqp_url: opts.rmq.clone(),
        prefetch_count: 1,
        enable_rso: false,
        rso_queue: String::new(),
        enable_valorant: false,
        valorant_queue: String::new(),
        enable_lol: false,
        lol_queue: String::new(),
        enable_tft: false,
        tft_queue: String::new(),
        enable_vod: false,
        vod_queue: String::new(),
        enable_csgo: false,
        csgo_queue: String::new(),
        enable_steam: false,
        steam_queue: String::new(),
        additional_queues: Some(vec![
            opts.queue.clone(),
        ])
    }, dst_pool.clone(), true).await.unwrap();

    let handler_itf = Arc::new(WowTaskHandler::new(src_pool.clone(), dst_pool.clone(), opts.encounter));

    for _i in 0..opts.threads {
        RabbitMqInterface::add_listener(rabbitmq.clone(), opts.queue.clone(), handler_itf.clone(), 1).await.unwrap();
    }

    loop {
        async_std::task::sleep(std::time::Duration::from_secs(10)).await;
    }
    Ok(())
}