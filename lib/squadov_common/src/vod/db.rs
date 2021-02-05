use crate::SquadOvError;
use crate::vod::VodAssociation;
use sqlx::{Executor, Transaction, Postgres};
use uuid::Uuid;

pub async fn get_vod_association<'a, T>(ex: T, uuid: &Uuid) -> Result<VodAssociation, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            VodAssociation,
            "
            SELECT *
            FROM squadov.vods
            WHERE video_uuid = $1
            ",
            uuid,
        )
            .fetch_one(ex)
            .await?
    )
}

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