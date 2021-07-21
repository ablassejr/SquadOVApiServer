use crate::SquadOvError;
use crate::vod::{VodAssociation, VodMetadata, VodSegmentId, VodThumbnail};
use sqlx::{Executor, Transaction, Postgres};
use uuid::Uuid;
use std::collections::HashMap;

pub async fn check_if_vod_public<'a, T>(ex: T, video_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM squadov.share_tokens AS st
                WHERE st.clip_uuid = $1
                UNION
                SELECT 1
                FROM squadov.share_tokens AS st
                INNER JOIN squadov.users AS u
                    ON u.id = st.user_id
                INNER JOIN squadov.vods AS v
                    ON v.match_uuid = st.match_uuid
                        AND v.user_uuid = u.uuid
                WHERE v.video_uuid = $1
            ) AS "exists!"
            "#,
            video_uuid
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}

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

pub async fn get_vod_metadata<'a, T>(ex: T, uuid: &Uuid, id: &str) -> Result<VodMetadata, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            VodMetadata,
            "
            SELECT *
            FROM squadov.vod_metadata
            WHERE video_uuid = $1
                AND id = $2
            ",
            uuid,
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_bulk_vod_metadata<'a, T>(ex: T, pairs: &[(Uuid, &str)]) -> Result<HashMap<(Uuid, String), VodMetadata>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let uuids: Vec<Uuid> = pairs.iter().map(|x| { x.0.clone() }).collect();
    let ids: Vec<String> = pairs.iter().map(|x| { x.1.to_string() }).collect();

    Ok(
        sqlx::query_as!(
            VodMetadata,
            "
            SELECT vm.*
            FROM UNNEST($1::UUID[], $2::VARCHAR[]) AS inp(uuid, id)
            INNER JOIN squadov.vod_metadata AS vm
                ON vm.video_uuid = inp.uuid
                    AND vm.id = inp.id
            ",
            &uuids,
            &ids,
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                ((x.video_uuid.clone(), x.id.clone()), x)
            })
            .collect()
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

pub async fn add_vod_thumbnail(tx: &mut Transaction<'_, Postgres>, vod_uuid: &Uuid, bucket: &str, segment: &VodSegmentId, width: i32, height: i32) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.vod_thumbnails (
            video_uuid,
            bucket,
            filepath,
            width,
            height
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5
        )
        ",
        vod_uuid,
        bucket,
        segment.get_fname(),
        width,
        height
    )
        .execute(tx)
        .await?;
    Ok(())
}

pub async fn get_vod_thumbnail<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Option<VodThumbnail>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            VodThumbnail,
            "
            SELECT *
            FROM squadov.vod_thumbnails
            WHERE video_uuid = $1
            ",
            video_uuid,
        )
            .fetch_optional(ex)
            .await?
    )
}
