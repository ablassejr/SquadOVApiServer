use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use crate::api::v1::UserResourcePath;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use squadov_common::{
    SquadOvError, SquadInvite,
    EmailTemplate, EmailUser,
};
use sqlx::{Transaction, Postgres, Row};
use serde::Deserialize;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct CreateSquadInviteInput {
    usernames: Vec<String>,
    emails: Vec<String>,
}

struct SquadOvInviteCreationHandle {
    invite_uuid: Uuid,
    username: Option<String>,
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

    async fn create_squad_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, inviter_user_id: i64, usernames: &[String], emails: &[String]) -> Result<HashMap<String, SquadOvInviteCreationHandle>, SquadOvError> {
        if usernames.is_empty() && emails.is_empty() {
            return Err(SquadOvError::BadRequest);
        }

        // Remove all usernames and emails already in the squad in question.
        // Note that in the case of usernames we force the existence of the username
        // via the inner join on the users table; however, in the case of emails
        // we allows sending invites to non-registered users. The query returns of
        // tuple of (user id, email) where user id can be an option if the user
        // is not yet registered.
        let mut filtered_users_emails: Vec<(Option<i64>, String)> = sqlx::query!(
            r#"
            SELECT u.id, u.email AS "email!"
            FROM UNNEST($2::VARCHAR[]) AS a(username)
            INNER JOIN squadov.users AS u
                ON u.username = a.username
            LEFT JOIN squadov.squad_role_assignments AS sra
                ON sra.user_id = u.id
            WHERE sra.squad_id != $1 OR sra.squad_id IS NULL
            UNION
            SELECT u.id, b.email AS "email!"
            FROM UNNEST($3::VARCHAR[]) AS b(email)
            LEFT JOIN squadov.users AS u
                ON u.email = b.email
            LEFT JOIN squadov.squad_role_assignments AS sra
                ON sra.user_id = u.id
            WHERE sra.squad_id != $1 OR sra.squad_id IS NULL
            "#,
            squad_id,
            usernames,
            emails
        )
            .fetch_all(&mut *tx)
            .await?
            .into_iter()
            .map(|x| { (x.id, x.email) })
            .collect();

        let filtered_users: Vec<Option<i64>> = filtered_users_emails.iter().map(|x| { x.0 }).collect();
        let filtered_emails: Vec<String> = filtered_users_emails.iter().map(|x| { x.1.clone() }).collect();
        filtered_users_emails.clear();

        // Create a squad invite for every user. Return a mapping of the email address to the invite uuid
        // so we can send invite emails as well.
        Ok(
            sqlx::query(
                r#"
                WITH inserted (email, user_id, invite_uuid) AS (
                    INSERT INTO squadov.squad_membership_invites (
                        squad_id,
                        inviter_user_id,
                        invite_time,
                        user_id,
                        email
                    )
                    SELECT $1, $2, NOW(), i.user_id, i.email
                    FROM UNNEST($3::BIGINT[], $4::VARCHAR[]) AS i(user_id, email)
                    RETURNING email, user_id, invite_uuid
                )
                SELECT i.email, i.invite_uuid, u.username
                FROM inserted AS i
                LEFT JOIN squadov.users AS u
                    ON u.id = i.user_id
                "#,
            )
                .bind(squad_id)
                .bind(inviter_user_id)
                .bind(filtered_users.as_slice())
                .bind(&filtered_emails)
                .fetch_all(&mut *tx)
                .await?
                .into_iter()
                .map(|x| {
                    (x.get(0), SquadOvInviteCreationHandle{
                        invite_uuid: x.get(1),
                        username: x.get(2),
                    })
                })
                .collect()
        )
    }

    pub async fn delete_squad_invite(&self, tx: &mut Transaction<'_, Postgres>, squad_id: i64, invite_uuid: &Uuid) -> Result<(), SquadOvError> {
        sqlx::query!(
            "
            DELETE FROM squadov.squad_membership_invites
            WHERE squad_id = $1 AND invite_uuid = $2 AND response_time IS NULL
            ",
            squad_id,
            invite_uuid
        )
            .execute(tx)
            .await?;
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
                smi.email,
                ur.username AS "username?",
                us.username AS "inviter_username"
            FROM squadov.squad_membership_invites AS smi
            INNER JOIN squadov.users AS us
                ON us.id = smi.inviter_user_id
            LEFT JOIN squadov.users AS ur
                ON ur.id = smi.user_id
            WHERE user_id = $1 AND response_time IS NULL
            "#,
            user_id
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| { x.hide_email() })
            .collect()
        )
    }

    pub async fn get_squad_invites(&self, squad_id: i64) -> Result<Vec<SquadInvite>, SquadOvError> {
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
                smi.email,
                ur.username AS "username?",
                us.username AS "inviter_username"
            FROM squadov.squad_membership_invites AS smi
            INNER JOIN squadov.users AS us
                ON us.id = smi.inviter_user_id
            LEFT JOIN squadov.users AS ur
                ON ur.id = smi.user_id
            WHERE smi.squad_id = $1 AND response_time IS NULL
            "#,
            squad_id
        )
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|x| { x.hide_email() })
            .collect()
        )
    }

    pub fn generate_invite_accept_reject_url(&self, invite_uuid: &Uuid) -> (String, String) {
        let base = format!(
            "{}/invite/{}",
            &self.config.squadov.app_url,
            invite_uuid
        );

        (
            format!(
                "{}/accept",
                &base
            ),
            format!(
                "{}/reject",
                &base
            ),
        )
    }
}

pub async fn create_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadSelectionInput>, data: web::Json<CreateSquadInviteInput>, req: HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = req.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let invites = app.create_squad_invite(&mut tx, path.squad_id, session.user.id, &data.usernames, &data.emails).await?;
    tx.commit().await?;

    // Now that we've tracked all the invites in the database, we can go about sending email invites for all the
    // users in question.
    match app.email.send_bulk_templated_email(&app.config.email.invite_template, invites.into_iter().map(|(email, invite)| {
        let (accept, reject) = app.generate_invite_accept_reject_url(&invite.invite_uuid);
        EmailTemplate{
            to: EmailUser{
                email: email,
                name: invite.username,
            },
            params: vec![
                (String::from("inviter_username"), session.user.username.clone()),
                (String::from("accept_url"), accept),
                (String::from("decline_url"), reject),
            ].into_iter().collect()
        }
    }).collect::<Vec<EmailTemplate>>()).await {
        Ok(_) => (),
        Err(err) => log::warn!("Failed to send squad invite emails: {:?}", err),
    };

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

pub async fn  get_all_squad_invites_handler(app : web::Data<Arc<api::ApiApplication>>, path : web::Path<super::SquadSelectionInput>) -> Result<HttpResponse, SquadOvError> {
    let invites = app.get_squad_invites(path.squad_id).await?;
    Ok(HttpResponse::Ok().json(&invites))
}

pub async fn revoke_squad_invite_handler(app : web::Data<Arc<api::ApiApplication>>, path: web::Path<super::SquadInviteInput>) -> Result<HttpResponse, SquadOvError> {
    let mut tx = app.pool.begin().await?;
    app.delete_squad_invite(&mut tx, path.squad_id, &path.invite_uuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().finish())
}