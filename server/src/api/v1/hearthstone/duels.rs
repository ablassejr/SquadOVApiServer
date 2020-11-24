use crate::api;
use chrono::{DateTime, Utc};
use squadov_common::SquadOvError;
use uuid::Uuid;
use sqlx::{Transaction, Postgres};

impl api::ApiApplication {
    pub async fn create_hearthstone_duels_run(&self, tx: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, user_id: i64, start_time: &DateTime<Utc>) -> Result<Uuid, SquadOvError> {
        let mc = self.create_new_match_collection(tx).await?;
        sqlx::query!(
            "
            INSERT INTO squadov.hearthstone_duels (
                collection_uuid,
                user_id,
                deck_id,
                creation_time
            )
            SELECT $1, $2, hdv.deck_id, $4
            FROM squadov.hearthstone_deck_versions AS hdv
            INNER JOIN squadov.hearthstone_match_user_deck AS hmud
                ON hmud.deck_version_id = hdv.version_id
            WHERE hmud.user_id = $2 AND hmud.match_uuid = $3
            ",
            mc.uuid,
            user_id,
            match_uuid,
            start_time,
        )
            .execute(tx)
            .await?;
        Ok(mc.uuid)
    }
}