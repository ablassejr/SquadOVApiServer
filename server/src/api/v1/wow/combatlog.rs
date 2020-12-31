use squadov_common::{
    SquadOvError,
    WoWCombatLogState,
    FullWoWCombatLogState,
    BlobResumableIdentifier
};
use actix_web::{web, HttpResponse, HttpRequest};
use crate::api;
use crate::api::auth::SquadOVSession;
use std::sync::Arc;
use uuid::Uuid;
use sqlx::{Executor, Postgres};

impl api::ApiApplication {
    pub async fn check_user_has_combat_log_for_match(&self, user_id: i64, match_uuid: &Uuid) -> Result<bool, SquadOvError> {
        Ok(
            sqlx::query!(
                "
                SELECT TRUE
                FROM squadov.wow_combat_logs AS wcl
                INNER JOIN squadov.wow_match_combat_log_association AS wmcla
                    ON wmcla.combat_log_uuid = wcl.uuid
                WHERE wcl.user_id = $1
                    AND wmcla.match_uuid = $2
                ",
                user_id,
                match_uuid
            )
                .fetch_optional(&*self.pool)
                .await?
                .is_some()
        )
    }

    pub async fn get_wow_combat_log(&self, combat_log_id: &Uuid) -> Result<FullWoWCombatLogState, SquadOvError> {
        let record = sqlx::query!(
            "
            SELECT
                wcl.combat_log_version,
                wcl.advanced_log,
                wcl.build_version,
                wcl.blob_uuid,
                bls.session_uri
            FROM squadov.wow_combat_logs AS wcl
            INNER JOIN squadov.blob_link_storage AS bls
                ON bls.uuid = wcl.blob_uuid
            WHERE wcl.uuid = $1
            ",
            combat_log_id
        )
            .fetch_one(&*self.pool)
            .await?;

        Ok(FullWoWCombatLogState{
            state: WoWCombatLogState{
                combat_log_version: record.combat_log_version,
                advanced_log: record.advanced_log,
                build_version: record.build_version,
            },
            blob: BlobResumableIdentifier{
                uuid: record.blob_uuid,
                session: record.session_uri,
            },
        })
    }

    async fn create_wow_combat_log<'a, T>(&self, ex: T, user_id: i64, state: &WoWCombatLogState, blob_uuid: &Uuid) -> Result<Uuid, SquadOvError>
    where
        T: Executor<'a, Database = Postgres>
    {
        let uuid = Uuid::new_v4();
        sqlx::query!(
            "
            INSERT INTO squadov.wow_combat_logs (
                uuid,
                user_id,
                combat_log_version,
                advanced_log,
                build_version,
                blob_uuid
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6
            )
            ",
            &uuid,
            user_id,
            &state.combat_log_version,
            state.advanced_log,
            &state.build_version,
            blob_uuid
        )
            .execute(ex)
            .await?;
        Ok(uuid)
    }
}

pub async fn create_wow_combat_log_handler(app : web::Data<Arc<api::ApiApplication>>, state: web::Json<WoWCombatLogState>, request : HttpRequest) -> Result<HttpResponse, SquadOvError> {
    let extensions = request.extensions();
    let session = match extensions.get::<SquadOVSession>() {
        Some(x) => x,
        None => return Err(SquadOvError::BadRequest)
    };

    let mut tx = app.pool.begin().await?;
    let blob = app.blob.begin_new_resumable_blob(&mut tx).await?;
    let uuid = app.create_wow_combat_log(&mut tx, session.user.id, &state, &blob.uuid).await?;
    tx.commit().await?;
    Ok(HttpResponse::Ok().json(uuid))
}