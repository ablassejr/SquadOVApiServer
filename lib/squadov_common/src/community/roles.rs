use sqlx::{Executor, Postgres};
use crate::{
    community::{
        CommunityRole,
    },
    SquadOvError,
};

pub async fn list_community_roles<'a, T>(ex: T, community_id: i64) -> Result<Vec<CommunityRole>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityRole,
            "
            SELECT *
            FROM squadov.community_roles
            WHERE community_id = $1
            ",
            community_id
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn delete_community_role<'a, T>(ex: T, community_id: i64, role_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    // What happens to role-less users???
    sqlx::query!(
        "
        DELETE FROM squadov.community_roles
        WHERE community_id = $1
            AND id = $2
        ",
        community_id,
        role_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn edit_community_role<'a, T>(ex: T, role: &CommunityRole) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.community_roles
        SET name = $3,
            can_manage = $4,
            can_moderate = $5,
            can_invite = $6,
            can_share = $7
        WHERE id = $1
            AND community_id = $2
        ",
        role.id,
        role.community_id,
        role.name,
        role.can_manage,
        role.can_moderate,
        role.can_invite,
        role.can_share,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_community_role<'a, T>(ex: T, role: &CommunityRole) -> Result<CommunityRole, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityRole,
            "
            INSERT INTO squadov.community_roles (
                community_id,
                name,
                can_manage,
                can_moderate,
                can_invite,
                can_share,
                is_default
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            )
            RETURNING *
            ",
            role.community_id,
            role.name,
            role.can_manage,
            role.can_moderate,
            role.can_invite,
            role.can_share,
            role.is_default,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn bulk_verify_community_roles<'a, T>(ex: T, community_id: i64, role_id: &[i64]) -> Result<bool, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            r#"
            SELECT COUNT(*) AS "count!"
            FROM squadov.community_roles AS cr
            WHERE cr.community_id = $1
                AND id = ANY($2)
            "#,
            community_id,
            role_id,
        )
            .fetch_one(ex)
            .await?
            .count == role_id.len() as i64
    )
}

pub async fn get_community_default_role<'a, T>(ex: T, community_id: i64) -> Result<CommunityRole, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityRole,
            "
            SELECT cr.*
            FROM squadov.community_roles AS cr
            WHERE cr.community_id = $1
                AND cr.is_default
            ",
            community_id,
        )
            .fetch_one(ex)
            .await?
    )
}