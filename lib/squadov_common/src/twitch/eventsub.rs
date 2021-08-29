use crate::{
    SquadOvError,
    twitch::api::TwitchSubscription,
    sql
};
use sqlx::{Executor, Postgres};

pub const TWITCH_CHANNEL_SUBSCRIBE: &'static str = "channel.subscribe";
pub const TWITCH_CHANNEL_UNSUB: &'static str = "channel.subscription.end";

pub async fn insert_twitch_eventsub<'a, T>(ex: T, id: &str, sub: &str, raw: serde_json::Value) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.twitch_event_subscriptions (
            id,
            sub,
            raw_data
        ) VALUES (
            $1,
            $2,
            $3
        )
        ",
        id,
        sub,
        raw
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn store_twitch_subs<'a, T>(ex: T, subs: &[TwitchSubscription]) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    if subs.is_empty() {
        return Ok(());
    }

    let mut query = vec![
        "
            INSERT INTO squadov.cached_twitch_subs (
                broadcast_user_id,
                viewer_user_id,
                tier
            )
            VALUES
        ".to_string()
    ];

    for s in subs {
        query.push(
            format!("
                (
                    {broadcast_user_id},
                    {viewer_user_id},
                    {tier}
                )
            ",
                broadcast_user_id=sql::sql_format_string(&s.broadcaster_id),
                viewer_user_id=sql::sql_format_string(&s.user_id),
                tier=sql::sql_format_string(&s.tier),
            )
        );
        query.push(",".to_string());       
    }

    query.truncate(query.len() - 1);
    sqlx::query(&query.join("")).execute(ex).await?;
    Ok(())
}

pub async fn delete_twitch_sub<'a, T>(ex: T, sub: &TwitchSubscription) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.cached_twitch_subs
        WHERE broadcast_user_id = $1
            AND viewer_user_id = $2
            AND tier = $3
        ",
        &sub.broadcaster_id,
        &sub.user_id,
        &sub.tier,
    )
        .execute(ex)
        .await?;
    Ok(())
}