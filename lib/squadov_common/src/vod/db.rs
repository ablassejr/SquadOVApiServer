use crate::{
    SquadOvError,
    vod::{VodAssociation, VodMetadata, VodSegmentId, VodThumbnail, StagedVodClip},
    SquadOvGames,
    sql,
};
use sqlx::{Executor, Transaction, Postgres};
use uuid::Uuid;
use std::collections::HashMap;

pub async fn mark_staged_clip_executed<'a, T>(ex: T, id: i64, clip_uuid: &Uuid) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.staged_clips
        SET execute_time = NOW(),
            clip_uuid = $2
        WHERE id = $1
        ",
        id,
        clip_uuid,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn bulk_add_video_metadata<'a, T>(ex: T, vod_uuid: &Uuid, data: &[VodMetadata]) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let mut sql : Vec<String> = Vec::new();
    sql.push(String::from("
        INSERT INTO squadov.vod_metadata (
            video_uuid,
            res_x,
            res_y,
            min_bitrate,
            avg_bitrate,
            max_bitrate,
            id,
            fps,
            bucket,
            session_id
        )
        VALUES
    "));

    for (idx, m) in data.iter().enumerate() {
        sql.push(format!("(
            '{video_uuid}',
            {res_x},
            {res_y},
            {min_bitrate},
            {avg_bitrate},
            {max_bitrate},
            '{id}',
            {fps},
            '{bucket}',
            {session_id}
        )",
            video_uuid=vod_uuid,
            res_x=m.res_x,
            res_y=m.res_y,
            min_bitrate=m.min_bitrate,
            avg_bitrate=m.avg_bitrate,
            max_bitrate=m.max_bitrate,
            id=m.id,
            fps=m.fps,
            bucket=m.bucket,
            session_id=sql::sql_format_option_string(&m.session_id),
        ));

        if idx != data.len() - 1 {
            sql.push(String::from(","))
        }
    }
    sqlx::query(&sql.join("")).execute(ex).await?;
    Ok(())
}

pub async fn associate_vod<'a, T>(ex: T, assoc: &VodAssociation) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.vods
        SET match_uuid = $1,
            user_uuid = $2,
            start_time = $3,
            end_time = $4,
            is_local = $5
        WHERE video_uuid = $6
        ",
        assoc.match_uuid,
        assoc.user_uuid,
        assoc.start_time,
        assoc.end_time,
        assoc.is_local,
        assoc.video_uuid,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn reserve_vod_uuid<'a, T>(ex: T, vod_uuid: &Uuid, container_format: &str, user_id: i64, is_clip: bool) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.vods (video_uuid, raw_container_format, user_uuid, is_clip)
        SELECT $1, $2, u.uuid, $4
        FROM squadov.users AS u
        WHERE u.id = $3
        ",
        vod_uuid,
        container_format,
        user_id,
        is_clip,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_clip<'a, T>(ex: T, clip_uuid: &Uuid, vod_uuid: &Uuid, user_id: i64, title: &str, description: &str, game: SquadOvGames, publish: bool) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.vod_clips (
            clip_uuid,
            parent_vod_uuid,
            clip_user_id,
            title,
            description,
            game,
            tm,
            published
        )
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            NOW(),
            $7
        )
        ",
        clip_uuid,
        vod_uuid,
        user_id,
        title,
        description,
        game as i32,
        publish,
    )
        .execute(ex)
        .await?;
    Ok(())
}
pub async fn get_staged_clips_for_vod<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<StagedVodClip>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            StagedVodClip,
            r#"
            SELECT *
            FROM squadov.staged_clips
            WHERE video_uuid = $1
                AND execute_time IS NULL
            "#,
            video_uuid
        )
            .fetch_all(ex)
            .await?
    )
}

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
                UNION
                SELECT 1
                FROM squadov.share_tokens AS st
                WHERE st.bulk_video_uuids @> ARRAY[$1]
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
            r#"
            SELECT
                vm.video_uuid AS "video_uuid!",
                vm.res_x AS "res_x!",
                vm.res_y AS "res_y!",
                vm.fps AS "fps!",
                vm.min_bitrate AS "min_bitrate!",
                vm.avg_bitrate AS "avg_bitrate!",
                vm.max_bitrate AS "max_bitrate!",
                vm.bucket AS "bucket!",
                vm.session_id AS "session_id",
                vm.id AS "id!",
                vm.has_fastify AS "has_fastify!",
                vm.has_preview AS "has_preview!"
            FROM UNNEST($1::UUID[], $2::VARCHAR[]) AS inp(uuid, id)
            INNER JOIN squadov.vod_metadata AS vm
                ON vm.video_uuid = inp.uuid
                    AND vm.id = inp.id
            "#,
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

pub async fn check_user_has_recorded_vod_for_match<'a, T>(ex: T, user_id: i64, match_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT *
                FROM squadov.vods AS v
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                WHERE u.id = $1
                    AND v.match_uuid = $2
                    AND v.is_clip = FALSE
                    AND v.is_local = FALSE
                    AND v.end_time IS NOT NULL
            ) AS "exists!"
            "#,
            user_id,
            match_uuid,
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}

pub async fn check_user_is_vod_owner<'a, T>(ex: T, user_id: i64, video_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT *
                FROM squadov.vods AS v
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                WHERE u.id = $1
                    AND v.video_uuid = $2
            ) AS "exists!"
            "#,
            user_id,
            video_uuid,
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}

pub async fn store_vod_md5<'a, T>(ex: T, video_uuid: &Uuid, hash: &str) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.vods
        SET md5 = $2
        WHERE video_uuid = $1
        ",
        video_uuid,
        hash,
    )
        .execute(ex)
        .await?;
    Ok(())
}