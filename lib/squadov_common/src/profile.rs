pub mod access;
pub mod data;

use crate::{
    SquadOvError,
    blob::BlobManagementClient,
};
use serde::Serialize;
use sqlx::{
    Executor,
    Postgres,
    postgres::PgPool,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::Arc;

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct UserProfileSocialLinksSerialized {
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub twitch: Option<String>,
    pub youtube: Option<String>,
    pub snapchat: Option<String>,
    pub twitter: Option<String>,
    pub tiktok: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct UserProfileMiscDataSerialized {
    pub description: String,
    pub profile_picture_url: Option<String>,
    pub cover_picture_url: Option<String>,
    pub links: UserProfileSocialLinksSerialized,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct UserProfileBasicSerialized {
    pub user_id: i64,
    pub link_slug: String,
    pub username: String,
    pub display_name: String,
    pub has_achievement_access: bool,
    pub has_match_access: bool,
    pub misc: Option<UserProfileMiscDataSerialized>,
    pub member_since: DateTime<Utc>,
    pub achievement_access: i32,
    pub match_access: i32,
    pub misc_access: i32,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct UserProfileBasicRaw {
    pub user_id: i64,
    pub link_slug: String,
    pub display_name: String,
    pub description: String,
    pub achievement_access: i32,
    pub match_access: i32,
    pub profile_picture_blob: Option<Uuid>,
    pub cover_picture_blob: Option<Uuid>,
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub twitch: Option<String>,
    pub youtube: Option<String>,
    pub snapchat: Option<String>,
    pub twitter: Option<String>,
    pub tiktok: Option<String>,
    pub misc_access: i32,
}

pub async fn get_user_profile_from_id<'a, T>(ex: T, id: i64) -> Result<UserProfileBasicRaw, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            UserProfileBasicRaw,
            "
            SELECT *
            FROM squadov.user_profiles
            WHERE user_id = $1
            ",
            id,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_user_profile_from_slug<'a, T>(ex: T, slug: &str) -> Result<UserProfileBasicRaw, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            UserProfileBasicRaw,
            "
            SELECT *
            FROM squadov.user_profiles
            WHERE link_slug = $1
            ",
            slug,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_user_profile_basic_serialized_with_requester(ex: &PgPool, profile: UserProfileBasicRaw, requester: Option<i64>, blob_client: Arc<BlobManagementClient>) -> Result<UserProfileBasicSerialized, SquadOvError> {
    // The main thing here is to do some access checks to determine whether or not to let certain data get passed back to the caller.
    let profile_access = access::get_user_profile_access(&*ex, &profile, requester).await?;
    let user_data = sqlx::query!(
        "
        SELECT username, registration_time
        FROM squadov.users
        WHERE id = $1
        ",
        profile.user_id,
    )
        .fetch_one(ex)
        .await?;
    Ok(
        UserProfileBasicSerialized {
            user_id: profile.user_id,
            link_slug: profile.link_slug,
            username: user_data.username,
            display_name: profile.display_name,
            has_achievement_access: profile_access.achievements,
            has_match_access: profile_access.matches,
            misc: if profile_access.misc {
                Some(UserProfileMiscDataSerialized {
                    description: profile.description,
                    profile_picture_url: if let Some(b) = profile.profile_picture_blob {
                        Some(blob_client.get_blob_url(&b).await?)
                    } else {
                        None
                    },
                    cover_picture_url: if let Some(b) = profile.cover_picture_blob {
                        Some(blob_client.get_blob_url(&b).await?)
                    } else {
                        None
                    },
                    links: UserProfileSocialLinksSerialized{
                        facebook: profile.facebook,
                        instagram: profile.instagram,
                        twitch: profile.twitch,
                        youtube: profile.youtube,
                        snapchat: profile.snapchat,
                        twitter: profile.twitter,
                        tiktok: profile.tiktok,
                    },
                })
            } else {
                None
            },
            member_since: user_data.registration_time.unwrap_or(Utc::now()),
            achievement_access: profile.achievement_access,
            match_access: profile.match_access,
            misc_access: profile.misc_access,
        }
    )
}