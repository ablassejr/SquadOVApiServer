use crate::{
    SquadOvError,
};

use sqlx::{
    Executor,
    Postgres,
};
use uuid::Uuid;

pub async fn update_user_profile_cover_photo_blob<'a, T>(ex: T, user_id: i64, b: Option<&Uuid>) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.user_profiles
        SET cover_picture_blob = $2
        WHERE user_id = $1
        ",
        user_id,
        b,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn update_user_profile_profile_photo_blob<'a, T>(ex: T, user_id: i64, b: Option<&Uuid>) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.user_profiles
        SET profile_picture_blob = $2
        WHERE user_id = $1
        ",
        user_id,
        b,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub struct UserProfileBasicUpdateData {
    pub description: Option<String>,
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub twitch: Option<String>,
    pub youtube: Option<String>,
    pub snapchat: Option<String>,
    pub twitter: Option<String>,
    pub tiktok: Option<String>,
    pub display_name: Option<String>,
}

impl std::default::Default for UserProfileBasicUpdateData {
    fn default() -> Self {
        Self {
            description: None,
            facebook: None,
            instagram: None,
            twitch: None,
            youtube: None,
            snapchat: None,
            twitter: None,
            tiktok: None,
            display_name: None,
        }
    }
}

pub async fn update_user_profile_basic_data<'a, T>(ex: T, user_id: i64, data: &UserProfileBasicUpdateData) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.user_profiles
        SET description = $2,
            facebook = $3,
            instagram = $4,
            twitch = $5,
            youtube = $6,
            snapchat = $7,
            twitter = $8,
            tiktok = $9,
            display_name = $10
        WHERE user_id = $1
        ",
        user_id,
        data.description.clone().unwrap_or(String::new()),
        data.facebook,
        data.instagram,
        data.twitch,
        data.youtube,
        data.snapchat,
        data.twitter,
        data.tiktok,
        data.display_name,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn check_if_match_shared_to_profile<'a, T>(ex: T, user_id: i64, match_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM squadov.vods AS v
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                INNER JOIN squadov.user_profile_vods AS upv
                    ON upv.user_id = u.id
                        AND upv.video_uuid = v.video_uuid
                WHERE u.id = $1 AND v.match_uuid = $2
                    AND v.is_clip = FALSE AND v.is_local = FALSE
                    AND v.end_time IS NOT NULL
            ) AS "exists!"
            "#,
            user_id,
            match_uuid
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}

pub async fn check_if_clip_shared_to_profile<'a, T>(ex: T, user_id: i64, clip_uuid: &Uuid) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM squadov.user_profile_vods
                WHERE user_id = $1 AND video_uuid = $2
            ) AS "exists!"
            "#,
            user_id,
            clip_uuid
        )
            .fetch_one(ex)
            .await?
            .exists
    )
}

pub async fn add_match_to_user_profile<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.user_profile_vods (
            user_id,
            video_uuid
        )
        SELECT $2, v.video_uuid
        FROM squadov.vods AS v
        INNER JOIN squadov.users AS u
            ON u.uuid = v.user_uuid
        WHERE u.id = $2
            AND v.match_uuid = $1
            AND v.is_clip = FALSE AND v.is_local = FALSE
            AND v.end_time IS NOT NULL
        ON CONFLICT DO NOTHING
        ",
        match_uuid,
        user_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn remove_match_from_user_profile<'a, T>(ex: T, match_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.user_profile_vods
        WHERE user_id = $2
            AND video_uuid IN (
                SELECT v.video_uuid
                FROM squadov.vods AS v
                INNER JOIN squadov.users AS u
                    ON u.uuid = v.user_uuid
                WHERE u.id = $2
                    AND v.match_uuid = $1
                    AND v.is_clip = FALSE AND v.is_local = FALSE
                    AND v.end_time IS NOT NULL
            )
        ",
        match_uuid,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn add_clip_to_user_profile<'a, T>(ex: T, video_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.user_profile_vods (
            user_id,
            video_uuid
        )
        VALUES (
            $2,
            $1
        )
        ON CONFLICT DO NOTHING
        ",
        video_uuid,
        user_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn remove_clip_from_user_profile<'a, T>(ex: T, video_uuid: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.user_profile_vods
        WHERE video_uuid = $1 AND user_id = $2
        ",
        video_uuid,
        user_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}