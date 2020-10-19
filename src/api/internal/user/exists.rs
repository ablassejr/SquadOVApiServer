use crate::common;
use crate::api;
use uuid::Uuid;
use sqlx;

impl api::ApiApplication {
    pub async fn internal_check_user_uuid_exists(&self,user_uuid: &Uuid) -> Result<bool, common::SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT EXISTS(
                SELECT *
                FROM squadov.users
                WHERE uuid = $1
            )
            "
        )
            .bind(user_uuid)
            .fetch_one(&*self.pool)
            .await?)
    }
}