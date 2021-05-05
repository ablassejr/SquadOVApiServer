use crate::SquadOvError;
use crate::csgo::{
    demo::CsgoDemo,
    gsi::CsgoGsiMatchState,
    schema::{CsgoView, CsgoCommonEventContainer},
};
use sqlx::{Transaction, Executor, Postgres};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub async fn create_csgo_view_for_user(ex: &mut Transaction<'_, Postgres>, user_id: i64, server: &str, start_time: &DateTime<Utc>, map: &str, mode: &str) -> Result<Uuid, SquadOvError> {
    Ok(
        sqlx::query!(
            "
            INSERT INTO squadov.csgo_match_views (
                view_uuid,
                user_id,
                game_server,
                start_time,
                map,
                mode
            ) VALUES (
                gen_random_uuid(),
                $1,
                $2,
                $3,
                $4,
                $5
            )
            RETURNING view_uuid
            ",
            user_id,
            server,
            start_time,
            map,
            mode,
        )
            .fetch_one(ex)
            .await?
            .view_uuid
    )
}

pub async fn find_csgo_view<'a, T>(ex: T, view_uuid: &Uuid) -> Result<CsgoView, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CsgoView,
            "
            SELECT *
            FROM squadov.csgo_match_views
            WHERE view_uuid = $1
            ",
            view_uuid,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn find_existing_csgo_match<'a, T>(ex: T, server: &str, start_time: &DateTime<Utc>, end_time: &DateTime<Utc>) -> Result<Option<Uuid>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT match_uuid
            FROM squadov.csgo_matches
            WHERE connected_server = $1
                AND tr && tstzrange($2, $3, '[]') 
            ",
            server,
            start_time,
            end_time
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                x.match_uuid
            })
    )
}

pub async fn create_csgo_match(ex: &mut Transaction<'_, Postgres>, match_uuid: &Uuid, server: &str, start_time: &DateTime<Utc>, end_time: &DateTime<Utc>) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        INSERT INTO squadov.csgo_matches (
            match_uuid,
            connected_server,
            tr
        ) VALUES (
            $1,
            $2,
            tstzrange($3, $4, '[]') 
        )
        ",
        match_uuid,
        server,
        start_time,
        end_time,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn finish_csgo_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_uuid: &Uuid, stop_time: &DateTime<Utc>, match_state: &CsgoGsiMatchState) -> Result<(), SquadOvError> {
    sqlx::query!(
        "
        UPDATE squadov.csgo_match_views
        SET match_uuid = $2,
            stop_time = $3
        WHERE view_uuid = $1
        ",
        view_uuid,
        match_uuid,
        stop_time
    )
        .execute(&mut *ex)
        .await?;

    store_csgo_gsi_events_for_view(ex, view_uuid, match_state).await?;
    Ok(())
}

pub async fn store_csgo_gsi_events_for_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, match_state: &CsgoGsiMatchState) -> Result<(), SquadOvError> {
    let common = CsgoCommonEventContainer::from_gsi(match_state)?;
    store_csgo_common_events_for_view(ex, view_uuid, &common).await?;
    sqlx::query!(
        "
        UPDATE squadov.csgo_match_views
        SET has_gsi = TRUE
        WHERE view_uuid = $1
        ",
        view_uuid
    )
        .execute(&mut *ex)
        .await?;
    Ok(())
}

pub async fn store_csgo_demo_events_for_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, demo: &CsgoDemo, ref_timestamp: &DateTime<Utc>) -> Result<(), SquadOvError> {
    let common = CsgoCommonEventContainer::from_demo(demo, ref_timestamp)?;
    store_csgo_common_events_for_view(ex, view_uuid, &common).await?;
    sqlx::query!(
        "
        UPDATE squadov.csgo_match_views
        SET has_demo = TRUE
        WHERE view_uuid = $1
        ",
        view_uuid
    )
        .execute(&mut *ex)
        .await?;
    Ok(())
}

pub async fn store_csgo_common_events_for_view(ex: &mut Transaction<'_, Postgres>, view_uuid: &Uuid, events: &CsgoCommonEventContainer) -> Result<(), SquadOvError> {
    let event_container_id = sqlx::query!(
        "
        INSERT INTO squadov.csgo_event_container (
            view_uuid,
            event_source
        )
        VALUES (
            $1,
            $2
        )
        RETURNING id
        ",
        view_uuid,
        events.event_source as i32,
    )
        .fetch_one(&mut *ex)
        .await?
        .id;

    Ok(())
}