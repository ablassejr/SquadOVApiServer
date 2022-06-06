use crate::{
    SquadOvError,
    combatlog::{
        CombatLogReport,
        CombatLogReportType,
    },
};
use async_trait::async_trait;
use sqlx::{
    Transaction, Postgres
};
use std::sync::Arc;
use rusoto_s3::{
    S3Client,
};

#[derive(Default)]
pub struct WowUserCharacterCacheReport {
    pub user_id: i64,
    pub build_version: String,
    pub unit_guid: String,
    pub unit_name: String,
    pub spec_id: i32,
    pub class_id: Option<i32>,
    pub items: Vec<i32>,
}


#[async_trait]
impl CombatLogReport for WowUserCharacterCacheReport {
    fn report_type(&self) -> CombatLogReportType {
        CombatLogReportType::Dynamic
    }

    async fn store_static_report(&self, _bucket: String, _partition: String, _s3: Arc<S3Client>) -> Result<(), SquadOvError> {
        Err(SquadOvError::BadRequest)
    }

    async fn store_dynamic_report(&self, tx: &mut Transaction<'_, Postgres>) -> Result<(), SquadOvError> {
        if self.items.is_empty() {
            // In this case, we didn't get full combatant information. Only update basic information (player name).
            sqlx::query!(
                "
                INSERT INTO squadov.wow_user_character_cache (
                    user_id,
                    unit_guid,
                    unit_name,
                    class_id,
                    cache_time,
                    build_version
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    NOW(),
                    $5
                ) ON CONFLICT (user_id, unit_guid) DO UPDATE SET
                    unit_name = EXCLUDED.unit_name,
                    class_id = EXCLUDED.class_id,
                    build_version = EXCLUDED.build_version
                ",
                &self.user_id,
                &self.unit_guid,
                &self.unit_name,
                self.class_id,
                &self.build_version,
            )
                .execute(tx)
                .await?;
        } else {
            sqlx::query!(
                "
                INSERT INTO squadov.wow_user_character_cache (
                    user_id,
                    unit_guid,
                    unit_name,
                    spec_id,
                    class_id,
                    items,
                    cache_time,
                    build_version
                ) VALUES (
                    $1,
                    $2,
                    $3,
                    $4,
                    $5,
                    $6,
                    NOW(),
                    $7
                ) ON CONFLICT (user_id, unit_guid) DO UPDATE SET
                    unit_name = EXCLUDED.unit_name,
                    class_id = EXCLUDED.class_id,
                    spec_id = EXCLUDED.spec_id,
                    items = EXCLUDED.items,
                    build_version = EXCLUDED.build_version
                ",
                &self.user_id,
                &self.unit_guid,
                &self.unit_name,
                &self.spec_id,
                self.class_id,
                &self.items,
                &self.build_version,
            )
                .execute(tx)
                .await?;
        }
        Ok(())
    }
}