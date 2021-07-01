use sqlx::{Executor, Postgres};
use crate::{
    community::{
        CommunityInvite
    },
    SquadOvError,
};
use uuid::Uuid;

pub async fn delete_community_invite<'a, T>(ex: T, code: &Uuid) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.community_invites
        WHERE code = $1
        ",
        code,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn create_community_invite<'a, T>(ex: T, invite: &CommunityInvite) -> Result<CommunityInvite, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityInvite,
            "
            INSERT INTO squadov.community_invites (
                code,
                community_id,
                inviter_user_id,
                num_uses,
                max_uses,
                expiration,
                created_tm
            ) VALUES (
                gen_random_uuid(),
                $1,
                $2,
                0,
                $3,
                $4,
                NOW()
            )
            RETURNING *
            ",
            invite.community_id,
            invite.inviter_user_id,
            invite.max_uses,
            invite.expiration,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_community_invite<'a, T>(ex: T, code: &Uuid) -> Result<CommunityInvite, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityInvite,
            "
            SELECT *
            FROM squadov.community_invites
            WHERE code = $1
            ",
            code,
        )
            .fetch_one(ex)
            .await?
    )
}

pub async fn get_user_community_invites<'a, T>(ex: T, community_id: i64, user_id: i64) -> Result<Vec<CommunityInvite>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityInvite,
            "
            SELECT *
            FROM squadov.community_invites
            WHERE community_id = $1
                AND inviter_user_id = $2
            ",
            community_id,
            user_id,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn increment_community_invite_usage<'a, T>(ex: T, code: &Uuid) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.community_invites
        SET num_uses = num_uses + 1
        WHERE code = $1
        ",
        code,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn record_community_invite_usage<'a, T>(ex: T, code: &Uuid, user_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.community_invite_usage (
            code,
            user_id,
            usage_tm
        ) VALUES (
            $1,
            $2,
            NOW()
        )
        ",
        code,
        user_id
    )
        .execute(ex)
        .await?;
    Ok(())
}