use actix_web::{web, HttpResponse};
use crate::api;
use std::sync::Arc;
use squadov_common::{SquadOvError};
use sqlx::{Transaction, Executor, Postgres, Row};
use serde::Deserialize;
use chrono::Utc;
use std::collections::HashSet;
use std::iter::FromIterator;

#[derive(Deserialize)]
pub struct CreateSquadInviteInput {
    users: Vec<i64>
}

impl api::ApiApplication {
    async fn create_squad_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, user_ids: &[i64]) -> Result<(), SquadOvError> {
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
                invite_time
            ) VALUES
            "
        ));

        for uid in user_ids {
            sql.push(format!("
                (
                    {},
                    {},
                    {}
                )",
                squad_id,
                uid,
                squadov_common::sql_format_time(&now),
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

    pub async fn accept_reject_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, user_id: i64, accepted: bool) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            UPDATE squadov.squad_membership_invites
            SET joined = $3,
                response_time = $4
            WHERE squad_id = $1 AND user_id = $2 AND response_time IS NULL
            ",
            squad_id,
            user_id,
            accepted,
            Utc::now(),
        )
            .execute(tx)
            .await?;
        Ok(())
    }

    pub async fn add_user_to_squad(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, user_id: i64) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            INSERT INTO squadov.squad_role_assignments (
                squad_id,
                user_id,
                squad_role
            )
            VALUES (
                $1,
                $2,
                'Member'
            )
            ",
            squad_id,
            user_id
        )
            .execute(tx)
            .await?;
        Ok(())
    }
}

pub async fn create_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>, data: web::Json<CreateSquadInviteInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.create_squad_invite(&mut tx, path.squad_id, &data.users).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn accept_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadInviteInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.accept_reject_invite(&mut tx, path.squad_id, path.user_id, true).await?;
    app.add_user_to_squad(&mut tx, path.squad_id, path.user_id).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn reject_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadInviteInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.accept_reject_invite(&mut tx, path.squad_id, path.user_id, false).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}