use actix_web::{web, HttpResponse};
use crate::api;
use squadov_common::SquadOvError;
use squadov_common::hearthstone::HearthstoneCardMetadata;
use std::sync::Arc;

impl api::ApiApplication {
    pub async fn bulk_get_hearthstone_cards_metadata(&self, card_ids: &[String]) -> Result<Vec<HearthstoneCardMetadata>, SquadOvError> {
        Ok(sqlx::query_as!(
            HearthstoneCardMetadata,
            r#"
            SELECT
                hc.card_id,
                hcn.string AS "name",
                hct_cost.val AS "cost",
                hct_rare.val AS "rarity"
            FROM squadov.hearthstone_cards AS hc
            INNER JOIN squadov.hearthstone_card_names AS hcn
                ON hcn.card_id = hc.card_id AND hcn.locale = 'en_US'
            INNER JOIN squadov.hearthstone_card_tags AS hct_cost
                ON hct_cost.card_id = hc.card_id AND hct_cost.tag = 48
                INNER JOIN squadov.hearthstone_card_tags AS hct_rare
                ON hct_rare.card_id = hc.card_id AND hct_rare.tag = 203
            WHERE hc.card_id = any($1)
            "#,
            card_ids
        )
            .fetch_all(&*self.pool)
            .await?)
    }
}

pub async fn bulk_get_hearthstone_cards_metadata_handler(app : web::Data<Arc<api::ApiApplication>>, card_ids : web::Json<Vec<String>>) -> Result<HttpResponse, SquadOvError> {
    let metadata = app.bulk_get_hearthstone_cards_metadata(&card_ids).await?;
    Ok(HttpResponse::Ok().json(&metadata))
}