use crate::common;
use crate::api;
use uuid::Uuid;
use sqlx;

impl api::ApiApplication {
    pub async fn reserve_vod_uuid(&self, vod_uuid: &Uuid) -> Result<(), common::SquadOvError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "
            INSERT INTO squadov.vods (video_uuid)
            VALUES ($1)
            ",
            vod_uuid
        )
            .execute(&mut tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }
}