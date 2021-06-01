use crate::api;
use squadov_common::{SquadOvError};
use uuid::Uuid;

impl api::ApiApplication {
    pub async fn list_squadov_accounts_can_access_vod(&self, vod_uuid: &Uuid) -> Result<Vec<i64>, SquadOvError> {
        // We need to go from the puuids of the users in the match to the user IDs stored by us
        // and then go from that to the users in the squads that can access the match.
        Ok(
            sqlx::query!(
                r#"
                SELECT DISTINCT user_id AS "user_id!"
                FROM squadov.view_share_connections_access_users
                WHERE video_uuid = $1
                "#,
                vod_uuid,
            )
                .fetch_all(&*self.pool)
                .await?
                .into_iter()
                .map(|x| x.user_id)
                .collect()
        )
    }

    pub async fn get_vod_owner(&self, vod_uuid: &Uuid) -> Result<Option<i64>, SquadOvError> {
        Ok(sqlx::query_scalar(
            "
            SELECT u.id
            FROM squadov.vods AS v
            INNER JOIN squadov.users AS u
                ON u.uuid = v.user_uuid
            WHERE v.video_uuid = $1
            ",
        )
            .bind(vod_uuid)
            .fetch_optional(&*self.pool)
            .await?)
    }
}