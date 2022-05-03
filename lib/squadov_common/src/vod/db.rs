use crate::{
    SquadOvError,
    vod::{
        VodAssociation,
        VodMetadata,
        VodManifest,
        VodTrack,
        VodSegment,
        VodSegmentId,
        VodThumbnail,
        VodClip,
        StagedVodClip,
        VodClipReactStats,
        self,
    },
    SquadOvGames,
    sql,
};
use sqlx::{Executor, Transaction, Postgres};
use uuid::Uuid;
use std::collections::HashMap;
use std::convert::TryFrom;

pub async fn delete_vod<'a, T>(ex: T, uuid: &Uuid) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.vods
        WHERE video_uuid = $1
        ",
        uuid
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn find_accessible_vods_in_match_for_user<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Vec<VodAssociation>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(sqlx::query_as!(
        VodAssociation,
        "
        SELECT DISTINCT v.*
        FROM squadov.vods AS v
        INNER JOIN squadov.users AS u
            ON u.uuid = v.user_uuid
        LEFT JOIN squadov.view_share_connections_access_users AS vau
            ON vau.video_uuid = v.video_uuid
                AND vau.match_uuid = $1
                AND vau.user_id = $2
        WHERE v.match_uuid = $1 
            AND (u.id = $2 OR vau.video_uuid IS NOT NULL)
            AND v.is_clip = FALSE
            AND (v.is_local = FALSE OR u.id = $2)
        ",
        match_uuid,
        user_id,
    )
        .fetch_all(ex)
        .await?)
}

pub async fn get_vod_id_from_match_user<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<Uuid, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT v.video_uuid
            FROM squadov.vods AS v
            INNER JOIN squadov.users AS u
                ON u.uuid = v.user_uuid
            WHERE v.match_uuid = $1
                AND u.id = $2
                AND v.is_clip = FALSE
            "#,
            match_uuid,
            user_id,
        )
            .fetch_one(ex)
            .await?
            .video_uuid
    )
}

pub async fn get_vod_game<'a, T>(ex: T, video_uuid: &Uuid) -> Result<SquadOvGames, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        SquadOvGames::try_from(
            sqlx::query!(
                r#"
                SELECT m.game
                FROM squadov.vods AS v
                INNER JOIN squadov.matches AS m
                    ON m.uuid = v.match_uuid
                WHERE v.video_uuid = $1
                "#,
                video_uuid,
            )
                .fetch_one(ex)
                .await?
                .game
                .unwrap_or(SquadOvGames::Unknown as i32)
        )?
    )
}

pub async fn get_vod_profiles<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT user_id
            FROM squadov.user_profile_vods
            WHERE video_uuid = $1
            ",
            video_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                x.user_id
            })
            .collect()
    )
}

pub async fn get_vod_favorites<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<(i64, String)>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT user_id, reason
            FROM squadov.user_favorite_vods
            WHERE video_uuid = $1
            ",
            video_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                (x.user_id, x.reason)
            })
            .collect()
    )
}

pub async fn get_vod_watchlist_ids<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT user_id
            FROM squadov.user_watchlist_vods
            WHERE video_uuid = $1
            ",
            video_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                x.user_id
            })
            .collect()
    )
}

pub async fn get_vod_shared_to_squads<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT DISTINCT dest_squad_id
            FROM squadov.share_match_vod_connections
            WHERE video_uuid = $1
            AND dest_squad_id IS NOT NULL
            ",
            video_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                x.dest_squad_id.unwrap()
            })
            .collect()
    )
}

pub async fn get_vod_shared_to_users<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT DISTINCT dest_user_id
            FROM squadov.share_match_vod_connections
            WHERE video_uuid = $1
                AND dest_user_id IS NOT NULL
            ",
            video_uuid
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                x.dest_user_id.unwrap()
            })
            .collect()
    )
}

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

pub async fn get_vod_manifest<'a, T>(ex: T, assoc: &VodAssociation) -> Result<VodManifest, SquadOvError>
where
    T: Executor<'a, Database = Postgres> + Copy
{
    let metadata = get_vod_metadata(ex, &assoc.video_uuid, "source").await?;
    let preview = if metadata.has_preview {
        Some(format!(
            "/v1/vod/{video_uuid}/{quality}/preview.mp4",
            video_uuid=&assoc.video_uuid,
            quality="source",
        ))
    } else {
        None
    };
    let container_format = String::from(if metadata.has_fastify {
        "mp4"
    } else {
        &assoc.raw_container_format
    });

    Ok(
        VodManifest{
            video_tracks: vec![
                VodTrack{
                    metadata: metadata.clone(),
                    segments: vec![VodSegment{
                        uri: format!("/v1/vod/{video_uuid}/{quality}/{segment}.{extension}",
                            video_uuid=assoc.video_uuid.clone(),
                            quality=&metadata.id,
                            segment=if metadata.has_fastify {
                                "fastify"
                            } else {
                                "video"
                            },
                            extension=&vod::container_format_to_extension(&container_format),
                        ),
                        // Duration is a placeholder - not really needed but will be useful once we get
                        // back to using semgnets.
                        duration: 0.0,
                        segment_start: 0.0,
                        mime_type: vod::container_format_to_mime_type(&container_format),
                    }],
                    preview: preview,
                }
            ]
        }
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

pub async fn get_vod_clip_from_assoc_manifest<'a, T>(ex: T, assoc: VodAssociation, manifest: VodManifest, user_id: i64) -> Result<Option<VodClip>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let base_data = sqlx::query!(
        r#"
        SELECT
            vc.*,
            u.username AS "clipper",
            COALESCE(rc.count, 0) AS "reacts!",
            COALESCE(cc.count, 0) AS "comments!",
            COALESCE(cv.count, 0) AS "views!",
            ufv.reason AS "favorite_reason?",
            uwv.video_uuid IS NOT NULL AS "is_watchlist!",
            COALESCE(JSONB_AGG(vvt.*) FILTER(WHERE vvt.video_uuid IS NOT NULL), '[]'::JSONB)  AS "tags!"
        FROM squadov.vod_clips AS vc
        INNER JOIN squadov.users AS u
            ON u.id = vc.clip_user_id
        LEFT JOIN squadov.view_clip_react_count AS rc
            ON rc.clip_uuid = vc.clip_uuid
        LEFT JOIN squadov.view_clip_comment_count AS cc
            ON cc.clip_uuid = vc.clip_uuid
        LEFT JOIN squadov.view_clip_view_count AS cv
            ON cv.clip_uuid = vc.clip_uuid
        LEFT JOIN squadov.user_favorite_vods AS ufv
            ON ufv.video_uuid = vc.clip_uuid
                AND ufv.user_id = $2
        LEFT JOIN squadov.user_watchlist_vods AS uwv
            ON uwv.video_uuid = vc.clip_uuid
                AND uwv.user_id = $2
        LEFT JOIN squadov.view_vod_tags AS vvt
            ON vvt.video_uuid = vc.clip_uuid
        WHERE vc.clip_uuid = $1
        GROUP BY vc.clip_uuid, vc.parent_vod_uuid, vc.clip_user_id, vc.title, vc.description, vc.game, vc.tm, vc.published, u.username, rc.count, cc.count, cv.count, ufv.reason, uwv.video_uuid
        ORDER BY vc.tm DESC
        "#,
        &assoc.video_uuid,
        user_id,
    )
        .fetch_optional(ex)
        .await?;

    Ok(
        base_data.map(|x| {
            VodClip{
                clip: assoc,
                manifest: manifest,
                title: x.title,
                description: x.description,
                clipper: x.clipper,
                game: SquadOvGames::try_from(x.game).unwrap(),
                tm: x.tm,
                views: x.views,
                reacts: x.reacts,
                comments: x.comments,
                favorite_reason: x.favorite_reason,
                is_watchlist: x.is_watchlist,
                access_token: None,
                tags: vod::condense_raw_vod_tags(serde_json::from_value::<Vec<vod::RawVodTag>>(x.tags).unwrap(), user_id),
                published: x.published,
            }
        })
    )
}

pub async fn get_raw_vod_tags<'a, T>(ex: T, video_uuid: &Uuid) -> Result<Vec<vod::RawVodTag>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let tags = sqlx::query!(
        r#"
        SELECT COALESCE(JSONB_AGG(vvt.*) FILTER(WHERE vvt.video_uuid IS NOT NULL), '[]'::JSONB)  AS "tags!"
        FROM squadov.view_vod_tags AS vvt
        WHERE vvt.video_uuid = $1
        "#,
        video_uuid
    )
        .fetch_one(ex)
        .await?
        .tags;
    
    Ok(serde_json::from_value::<Vec<vod::RawVodTag>>(tags)?)
}

pub async fn get_vod_clip_react_stats<'a, T>(ex: T, video_uuids: &[Uuid]) -> Result<HashMap<Uuid, VodClipReactStats>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let base_data = sqlx::query_as!(
        VodClipReactStats,
        r#"
        SELECT
            vc.clip_uuid AS "video_uuid!",
            COALESCE(rc.count, 0) AS "reacts!",
            COALESCE(cc.count, 0) AS "comments!",
            COALESCE(cv.count, 0) AS "views!"
        FROM squadov.vod_clips AS vc
        INNER JOIN squadov.users AS u
            ON u.id = vc.clip_user_id
        LEFT JOIN squadov.view_clip_react_count AS rc
            ON rc.clip_uuid = vc.clip_uuid
        LEFT JOIN squadov.view_clip_comment_count AS cc
            ON cc.clip_uuid = vc.clip_uuid
        LEFT JOIN squadov.view_clip_view_count AS cv
            ON cv.clip_uuid = vc.clip_uuid
        WHERE vc.clip_uuid = ANY($1)
        "#,
        video_uuids,
    )
        .fetch_all(ex)
        .await?;

    Ok(
        base_data.into_iter().map(|x| {
            ( x.video_uuid.clone(), x)
        }).collect()
    )
}