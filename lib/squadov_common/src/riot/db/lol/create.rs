use crate::{
    SquadOvError,
    matches
};
use sqlx::{Transaction, Postgres};
use uuid::Uuid;

async fn link_match_uuid_to_lol_match(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, platform: &str, game_id: i64) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.lol_matches (
            match_uuid,
            platform,
            match_id
        )
        VALUES (
            $1,
            $2,
            $3
        )
        ",
        match_uuid,
        platform,
        game_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_or_get_match_uuid_for_lol_match(ex: &mut Transaction<'_, Postgres>, platform: &str, game_id: i64) -> Result<Uuid, SquadOvError> {
    Ok(match super::get_lol_match_uuid_if_exists(&mut *ex, platform, game_id).await? {
        Some(x) => x,
        None => {
            let match_uuid = matches::create_new_match(&mut *ex).await?;
            link_match_uuid_to_lol_match(&mut *ex, &match_uuid, platform, game_id).await?;
            match_uuid
        }
    })
}