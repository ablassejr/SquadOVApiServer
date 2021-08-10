use serde::{Serialize, Deserialize};
use crate::SquadOvError;
use sqlx::{Executor, Postgres};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct OperatingSystem {
    pub name: String,
    pub major_version: String,
    pub minor_version: String,
    pub edition: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct CPU {
    pub vendor: String,
    pub brand: String,
    pub clock: i64,
    pub cores: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct Monitor {
    pub manufacturer: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub year: i32,
    pub refresh: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct GPU {
    pub name: String,
    pub memory_bytes: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct Display {
    pub gpus: Vec<GPU>,
    pub monitors: Vec<Monitor>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all="camelCase")]
pub struct Hardware {
    pub os: OperatingSystem,
    pub cpu: CPU,
    pub display: Display,
    pub ram_kb: i64,
}

pub async fn get_hardware_for_user<'a, T>(ex: T, user_id: i64) -> Result<Option<Hardware>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let data = sqlx::query!(
        "
        SELECT *
        FROM squadov.user_hardware_specs
        WHERE user_id = $1
        ",
        user_id,
    )
        .fetch_optional(ex)
        .await?;

    if let Some(d) = data {
        Ok(Some(Hardware{
            os: serde_json::from_value::<OperatingSystem>(d.os)?,
            cpu: serde_json::from_value::<CPU>(d.cpu)?,
            display: serde_json::from_value::<Display>(d.display)?,
            ram_kb: d.ram_kb,
        }))
    } else {
        Ok(None)
    }
}

pub async fn store_hardware_for_user<'a, T>(ex: T, user_id: i64, hw: Hardware) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.user_hardware_specs (
            user_id,
            os,
            cpu,
            display,
            ram_kb
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5
        )
        ON CONFLICT (user_id) DO UPDATE SET
            os=EXCLUDED.os,
            cpu=EXCLUDED.cpu,
            display=EXCLUDED.display,
            ram_kb=EXCLUDED.ram_kb
        ",
        user_id,
        serde_json::to_value(hw.os)?,
        serde_json::to_value(hw.cpu)?,
        serde_json::to_value(hw.display)?,
        hw.ram_kb,
    )
        .execute(ex)
        .await?;
    Ok(())
}