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
            tiktok = $9
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
    )
        .execute(ex)
        .await?;
    Ok(())
}