use chrono::{DateTime, Utc, prelude::NaiveTime};

#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatSparData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
    kill: f64,
    ttk: f64,
    acc: f64
}

#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatDetectionData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
}


#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatDecisionshotData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
}

#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatTrackData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
}

#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatErbData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
}

#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatLinetraceData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
}

#[derive(sqlx::FromRow)]
#[derive(juniper::GraphQLObject)]
pub struct AimlabStatPentakillData {
    date: Option<DateTime<Utc>>,
    datetime: Option<DateTime<Utc>>,
    time: Option<NaiveTime>,
    version: Option<String>,
    id: Option<f64>,
    score: f64,
}