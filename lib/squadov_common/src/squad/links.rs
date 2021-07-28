use crate::SquadOvError;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::{Executor, Postgres};

#[derive(Clone, Debug)]
pub struct SquadInviteLink {
    pub id: i64,
    pub squad_id: i64,
    pub user_id: i64,
    pub create_time: DateTime<Utc>,
    pub expire_time: Option<DateTime<Utc>>,
    pub use_count: i32,
    pub max_uses: Option<i32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all="camelCase")]
pub struct PublicSquadInviteLink {
    pub id: String,
    pub squad_id: i64,
    pub user_id: i64,
    pub create_time: DateTime<Utc>,
    pub expire_time: Option<DateTime<Utc>>,
    pub use_count: i32,
    pub max_uses: Option<i32>,
    pub link: String,
}

pub async fn get_squad_invite_link_from_id<'a, T>(ex: T, id: i64) -> Result<SquadInviteLink, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            SquadInviteLink,
            r#"
            SELECT sil.*, COUNT(slu.link_id)::INTEGER AS "use_count!"
            FROM squadov.squad_invite_links AS sil
            LEFT JOIN squadov.squad_invite_link_usage AS slu
                ON slu.link_id = sil.id
            WHERE sil.id = $1
            GROUP BY sil.id, sil.squad_id, sil.user_id, sil.create_time, sil.expire_time, sil.max_uses
            "#,
            id
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_squad_invite_links_for_user<'a, T>(ex: T, squad_id: i64, user_id: i64) -> Result<Vec<SquadInviteLink>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            SquadInviteLink,
            r#"
            SELECT sil.*, COUNT(slu.link_id)::INTEGER AS "use_count!"
            FROM squadov.squad_invite_links AS sil
            LEFT JOIN squadov.squad_invite_link_usage AS slu
                ON slu.link_id = sil.id
            WHERE sil.squad_id = $1 AND sil.user_id = $2
            GROUP BY sil.id, sil.squad_id, sil.user_id, sil.create_time, sil.expire_time, sil.max_uses
            "#,
            squad_id,
            user_id,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn create_default_squad_invite_link_for_user<'a, T>(ex: T, squad_id: i64, user_id: i64) -> Result<SquadInviteLink, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let x = sqlx::query!(
        r#"
        INSERT INTO squadov.squad_invite_links (
            squad_id,
            user_id,
            create_time
        )
        VALUES (
            $1,
            $2,
            NOW()
        )
        RETURNING *
        "#,
        squad_id,
        user_id,
    )
        .fetch_one(ex)
        .await?;

    Ok(
        SquadInviteLink {
            id: x.id,
            squad_id: x.squad_id,
            user_id: x.user_id,
            create_time: x.create_time,
            expire_time: x.expire_time,
            use_count: 0,
            max_uses: x.max_uses,
        }
    )
}

pub async fn modify_squad_invite<'a, T>(ex: T, link: SquadInviteLink) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.squad_invite_links
        SET expire_time = $4,
            max_uses = $5
        WHERE id = $1
            AND squad_id = $2
            AND user_id = $3
        ",
        link.id,
        link.squad_id,
        link.user_id,
        link.expire_time,
        link.max_uses
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn delete_squad_invite<'a, T>(ex: T, link_id: i64, squad_id: i64, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.squad_invite_links
        WHERE id = $1
            AND squad_id = $2
            AND user_id = $3
        ",
        link_id,
        squad_id,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn mark_squad_invite_link_used<'a, T>(ex: T, link_id: i64, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.squad_invite_link_usage (
            link_id,
            user_id,
            usage_time
        )
        VALUES (
            $1,
            $2,
            NOW()
        )
        ",
        link_id,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}
