pub mod rabbitmq;
pub mod api;
pub mod db;

use crate::SquadOvError;
use serde::Serialize;
use sqlx::{Transaction, Postgres};

#[derive(Serialize)]
pub struct SteamAccount {
    pub steam_id: i64,
    pub name: String,
    pub profile_image_url: Option<String>,
}

pub async fn link_steam_id_to_user(ex: &mut Transaction<'_, Postgres>, steam_id: i64, user_id: i64) -> Result<(), SquadOvError> {
    sqlx::query!("
        INSERT INTO squadov.steam_user_links (
            steam_id,
            user_id
        ) VALUES (
            $1,
            $2
        )
    ",
        steam_id,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}