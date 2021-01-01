use actix_web::{web, HttpResponse};
use crate::api;
use squadov_common::SquadOvError;
use std::sync::Arc;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    #[serde(deserialize_with="squadov_common::parse_utc_time_from_milliseconds")]
    start: Option<DateTime<Utc>>,
    #[serde(deserialize_with="squadov_common::parse_utc_time_from_milliseconds")]
    end: Option<DateTime<Utc>>
}

#[derive(Serialize)]
pub struct UserAnalyticsDatum {
    tm: DateTime<Utc>,
    session: i64,
    endpoint: i64,
    vod: i64
}

pub async fn get_daily_analytics_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<AnalyticsQuery>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        sqlx::query_as!(
            UserAnalyticsDatum,
            r#"
            WITH series(tm) AS (
                SELECT *
                FROM generate_series(
                    DATE_TRUNC('day', $1::TIMESTAMPTZ),
                    DATE_TRUNC('day', $2::TIMESTAMPTZ),
                    INTERVAL '1 day'
                )
            )
            SELECT
                s.tm AS "tm!",
                COALESCE(das.users, 0) AS "session!",
                COALESCE(dae.users, 0) AS "endpoint!",
                COALESCE(dav.users, 0) AS "vod!"
            FROM series AS s
            LEFT JOIN squadov.view_daily_active_user_sessions AS das
                ON das.tm = s.tm
            LEFT JOIN squadov.view_daily_active_user_endpoint AS dae
                ON dae.tm = s.tm
            LEFT JOIN squadov.view_daily_active_vod_users AS dav
                ON dav.tm = s.tm
            ORDER BY s.tm ASC
            "#,
            query.start.unwrap_or(chrono::MIN_DATETIME),
            query.end.unwrap_or(chrono::MAX_DATETIME)
        )
            .fetch_all(&*app.pool)
            .await?
    ))
}

pub async fn get_monthly_analytics_handler(app : web::Data<Arc<api::ApiApplication>>, query: web::Query<AnalyticsQuery>) -> Result<HttpResponse, SquadOvError> {
    Ok(HttpResponse::Ok().json(
        sqlx::query_as!(
            UserAnalyticsDatum,
            r#"
            WITH series(tm) AS (
                SELECT *
                FROM generate_series(
                    DATE_TRUNC('month', $1::TIMESTAMPTZ),
                    DATE_TRUNC('month', $2::TIMESTAMPTZ),
                    INTERVAL '1 month'
                )
            )
            SELECT
                s.tm AS "tm!",
                COALESCE(das.users, 0) AS "session!",
                COALESCE(dae.users, 0) AS "endpoint!",
                COALESCE(dav.users, 0) AS "vod!"
            FROM series AS s
            LEFT JOIN squadov.view_monthly_active_user_sessions AS das
                ON das.tm = s.tm
            LEFT JOIN squadov.view_monthly_active_user_endpoint AS dae
                ON dae.tm = s.tm
            LEFT JOIN squadov.view_monthly_active_vod_users AS dav
                ON dav.tm = s.tm
            ORDER BY s.tm ASC
            "#,
            query.start.unwrap_or(chrono::MIN_DATETIME),
            query.end.unwrap_or(chrono::MAX_DATETIME)
        )
            .fetch_all(&*app.pool)
            .await?
    ))
}