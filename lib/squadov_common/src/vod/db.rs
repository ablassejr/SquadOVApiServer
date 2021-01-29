use crate::SquadOvError;
use sqlx::{Transaction, Postgres};
use uuid::Uuid;

pub async fn mark_vod_as_fastify(tx : &mut Transaction<'_, Postgres>, vod_uuid: &Uuid) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        UPDATE squadov.vod_metadata
        SET has_fastify = true
        WHERE video_uuid = $1
        ",
        vod_uuid
    )
        .execute(tx)
        .await?;
    Ok(())
}

pub async fn mark_vod_with_preview(tx : &mut Transaction<'_, Postgres>, vod_uuid: &Uuid) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        UPDATE squadov.vod_metadata
        SET has_preview = true
        WHERE video_uuid = $1
        ",
        vod_uuid
    )
        .execute(tx)
        .await?;
    Ok(())
}