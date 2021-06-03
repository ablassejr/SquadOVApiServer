use serde::{Serialize, Deserialize};
use crate::{
    SquadOvError,
    SquadOvGames,
};
use sqlx::{Executor, Postgres};
use std::convert::TryFrom;

#[derive(Serialize,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AutoShareConnection {
    pub id: i64,
    pub source_user_id: i64,
    pub can_share: bool,
    pub can_clip: bool,
    pub dest_user_id: Option<i64>,
    pub dest_squad_id: Option<i64>,
    #[serde(default)]
    pub games: Vec<SquadOvGames>,
}

pub async fn get_auto_share_connections_for_user<'a, T>(ex: T, user_id: i64) -> Result<Vec<AutoShareConnection>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT uas.*, ARRAY_AGG(asg.game) AS "games!"
            FROM squadov.user_autosharing_settings AS uas
            INNER JOIN squadov.user_autosharing_settings_games AS asg
                ON asg.id = uas.id
            WHERE uas.source_user_id = $1
            GROUP BY
                uas.id,
                uas.source_user_id,
                uas.can_share,
                uas.can_clip,
                uas.dest_user_id,
                uas.dest_squad_id
            "#,
            user_id,
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                Ok(AutoShareConnection{
                    id: x.id,
                    source_user_id: x.source_user_id,
                    can_share: x.can_share,
                    can_clip: x.can_clip,
                    dest_user_id: x.dest_user_id,
                    dest_squad_id: x.dest_squad_id,
                    games: x.games.into_iter().map(|x| {
                        Ok(SquadOvGames::try_from(x)?)
                    }).collect::<Result<Vec<SquadOvGames>, SquadOvError>>()?
                })
            })
            .collect::<Result<Vec<AutoShareConnection>, SquadOvError>>()?
    )
}

pub async fn create_auto_share_connection_for_user<'a, T>(ex: T, conn: AutoShareConnection, user_id: i64) -> Result<AutoShareConnection, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let x = sqlx::query!(
        "
        INSERT INTO squadov.user_autosharing_settings (
            source_user_id,
            dest_user_id,
            dest_squad_id,
            can_share,
            can_clip
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5
        )
        RETURNING *
        ",
        user_id,
        conn.dest_user_id,
        conn.dest_squad_id,
        conn.can_share,
        conn.can_clip,
    )
        .fetch_one(ex)
        .await?;

    Ok(
        AutoShareConnection{
            id: x.id,
            source_user_id: x.source_user_id,
            can_share: x.can_share,
            can_clip: x.can_clip,
            dest_user_id: x.dest_user_id,
            dest_squad_id: x.dest_squad_id,
            games: conn.games,
        }
    )
}

pub async fn link_auto_share_connection_to_games_for_user<'a, T>(ex: T, user_id: i64, conn_id: i64, games: &[SquadOvGames]) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.user_autosharing_settings_games (
            id,
            game
        )
        SELECT uas.id, inp.game
        FROM UNNEST($3::INTEGER[]) AS inp(game)
        CROSS JOIN (
            SELECT *
            FROM squadov.user_autosharing_settings
            WHERE id = $1 AND source_user_id = $2
        ) AS uas
        ",
        conn_id,
        user_id,
        &games.iter().map(|x| {
            *x as i32
        }).collect::<Vec<i32>>(),
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn delete_auto_share_connection_to_games_for_user<'a, T>(ex: T, user_id: i64, conn_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.user_autosharing_settings_games AS asg 
        USING squadov.user_autosharing_settings AS uas
        WHERE uas.id = asg.id
            AND asg.id = $1
            AND uas.source_user_id = $2
        ",
        conn_id,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn delete_auto_share_connection_for_user<'a, T>(ex: T, user_id: i64, conn_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.user_autosharing_settings
        WHERE id = $1 AND source_user_id = $2
        ",
        conn_id,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn edit_auto_share_connection_for_user<'a, T>(ex: T, conn: &AutoShareConnection, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.user_autosharing_settings
        SET can_clip = $3,
            can_share = $4
        WHERE id = $1 AND source_user_id = $2
        ",
        conn.id,
        user_id,
        conn.can_clip,
        conn.can_share
    )
        .execute(ex)
        .await?;
    Ok(())
}