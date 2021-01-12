use crate::{
    SquadOvError,
};
use sqlx::{Transaction, Postgres};
use uuid::Uuid;

pub async fn create_or_get_match_uuid_for_tft_match(ex: &mut Transaction<'_, Postgres>, platform: &str, game_id: i64) -> Result<Uuid, SquadOvError> {
    Err(SquadOvError::NotFound)
}