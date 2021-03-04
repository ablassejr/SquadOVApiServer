mod riot;

pub use riot::*;

use crate::api;
use crate::api::auth::SquadOVUser;
use squadov_common::{
    SquadOvError,
    EmailTemplate, EmailUser,
};
use uuid::Uuid;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

impl api::ApiApplication {
    pub async fn user_id_to_uuid(&self, user_id: i64) -> Result<Uuid, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT uuid
                FROM squadov.users AS u
                WHERE u.id = $1
                ",
                user_id,
            )
                .fetch_one(&*self.pool)
                .await?
                .uuid
        )
    }

    pub async fn get_user_uuid_to_user_id_map(&self, uuids: &[Uuid]) -> Result<HashMap<Uuid, i64>, SquadOvError> {
        Ok(sqlx::query!(
            "
            SELECT u.uuid, u.id
            FROM squadov.users AS u
            WHERE u.uuid = any($1)
            ",
            uuids
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| { (x.uuid, x.id) } )
            .collect())
    }


    pub async fn update_user_registration_time(&self, user_id: i64, tm: &DateTime<Utc>) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.users
            SET registration_time = $2
            WHERE id = $1
            ",
            user_id,
            tm,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn send_welcome_email_to_user(&self, user: &SquadOVUser) -> Result<(), SquadOvError> {
        self.email.send_bulk_templated_email(&self.config.email.welcome_template, vec![
            EmailTemplate{
                to: EmailUser{
                    email: user.email.clone(),
                    name: Some(user.username.clone()),
                },
                params: vec![
                    (String::from("product_url"), String::from("https://www.squadov.gg")),
                    (String::from("product_name"), String::from("SquadOV")),
                    (String::from("username"), user.username.clone()),
                ].into_iter().collect()
            }
        ]).await?;
        sqlx::query!(
            "
            UPDATE squadov.users
            SET welcome_sent = TRUE
            WHERE id = $1
            ",
            user.id,
        )
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
}