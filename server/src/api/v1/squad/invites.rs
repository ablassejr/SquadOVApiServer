use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use crate::api::v1::UserResourcePath;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use squadov_common::{SquadOvError, SquadInvite};
use sqlx::{Transaction, Executor, Postgres, Row};
use serde::Deserialize;
use chrono::Utc;
use std::collections::HashSet;
use std::iter::FromIterator;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateSquadInviteInput {
    users: Vec<i64>
}

impl api::ApiApplication {
    pub async fn get_squad_invite_user(&self, invite_uuid: &Uuid) -> Result<i64, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT user_id
            FROM squadov.squad_membership_invites
            WHERE invite_uuid = $1
            "
        )
            .bind(invite_uuid)
            .fetch_one(&*self.pool)
            .await?)
    }

    async fn create_squad_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, inviter_user_id: i64, user_ids: &[i64]) -> Result<(), SquadOvError> {
        if user_ids.is_empty() {
            return Ok(());
        }

        // Filter out user ids that already are already part of the Squad.
        let existing_user_ids: Vec<i64> = tx.fetch_all(
            sqlx::query(
                "
                SELECT user_id
                FROM squadov.squad_role_assignments
                WHERE squad_id = $1 AND user_id = any($2)
                "
            )
                .bind(squad_id)
                .bind(user_ids)
        ).await?.into_iter().map(|x| {
            x.get(0)
        }).collect();
        let existing_user_ids: HashSet<i64> = HashSet::from_iter(existing_user_ids.into_iter());
        let user_ids: Vec<i64> = user_ids.iter().cloned().filter(|x| {
            !existing_user_ids.contains(x)
        }).collect();

        if user_ids.is_empty() {
            return Ok(());
        }

        let mut sql: Vec<String> = Vec::new();
        let now = Utc::now();

        sql.push(String::from(
            "
            INSERT INTO squadov.squad_membership_invites(
                squad_id,
                user_id,
                invite_time,
                inviter_user_id
            ) VALUES
            "
        ));

        for uid in user_ids {
            sql.push(format!("
                (
                    {},
                    {},
                    {},
                    {}
                )",
                squad_id,
                uid,
                squadov_common::sql_format_time(&now),
                inviter_user_id,
            ));
            sql.push(String::from(","));
        }
        sql.truncate(sql.len() - 1);
        sqlx::query(&sql.join(" ")).execute(tx).await?;

        // TODO #13: Send squad invite emails once we've successfully tracked them in the database.
        // Any invite that doesn't get sent (e.g. an error occurs during sending) should be ignored as
        // we should just force the user to deal with an unreceived invite (email) and resending the invite
        // if necessary.
        Ok(())
    }

    pub async fn accept_reject_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, invite_uuid: &Uuid, accepted: bool) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.squad_membership_invites
            SET joined = $3,
                response_time = $4
            WHERE squad_id = $1 AND invite_uuid = $2 AND response_time IS NULL
            RETURNING invite_uuid
            ",
            squad_id,
            invite_uuid,
            accepted,
            Utc::now(),
        )
            // Do a fetch one here to error if we try to accept/reject an already used invite.
            .fetch_one(tx)
            .await?;
        Ok(())
    }

    pub async fn add_user_to_squad_from_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, invite_uuid: &Uuid) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.squad_role_assignments (
                squad_id,
                user_id,
                squad_role
            )
            SELECT $1, user_id, 'Member'
            FROM squadov.squad_membership_invites
            WHERE squad_id = $1 AND invite_uuid = $2
            ",
            squad_id,
            invite_uuid
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn get_user_squad_invites(&self, user_id: i64) -> Result<Vec<SquadInvite>, SquadOvError> {
        Ok(sqlx::query_as!(
            SquadInvite,
            r#"
            SELECT
                smi.squad_id,
                smi.user_id,
                smi.joined,
                smi.response_time,
                smi.invite_time,
                smi.invite_uuid,
                us.username AS "inviter_username"
            FROM squadov.squad_membership_invites AS smi
            INNER JOIN squadov.users AS us
                ON us.id = smi.inviter_user_id
            WHERE user_id = $1 AND response_time IS NULL
            "#,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?)
    }
}

pub async fn create_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>, data: web::Json<CreateSquadInviteInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    app.create_squad_invite(&mut tx, path.squad_id, session.user.id, &data.users).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn accept_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadInviteInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.accept_reject_invite(&mut tx, path.squad_id, &path.invite_uuid, true).await?;
    app.add_user_to_squad_from_invite(&mut tx, path.squad_id, &path.invite_uuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn reject_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadInviteInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.accept_reject_invite(&mut tx, path.squad_id, &path.invite_uuid, false).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get_user_squad_invites_handler(app : web::Data<Arc<api::ApiApplication>>, path : web::Path<UserResourcePath>) -> Result<HttpResponse, SquadOvError> {
    let invites = app.get_user_squad_invites(path.user_id).await?;
    Ok(HttpResponse::Ok().json(&invites))
}