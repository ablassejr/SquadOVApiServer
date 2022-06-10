use sqlx::{Executor, Postgres};
use crate::{
    community::{
        SquadOvCommunity,
        CommunityRole,
        CommunitySecurityLevel,
        CommunityUser,
    },
    SquadOvError,
    user::SquadOVUser,
};
use std::convert::TryFrom;

pub async fn find_communities_for_user<'a, T>(ex: T, user_id: i64) -> Result<Vec<SquadOvCommunity>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT c.*
            FROM squadov.community_membership AS cm
            INNER JOIN squadov.communities AS c
                ON c.id = cm.community_id 
            WHERE cm.user_id = $1
            ",
            user_id
        )
            .fetch_all(ex)
            .await?
            .into_iter()
            .map(|x| {
                Ok(SquadOvCommunity{
                    id: x.id,
                    name: x.name,
                    slug: x.slug,
                    create_tm: x.create_tm,
                    creator_user_id: x.creator_user_id,
                    security_level: CommunitySecurityLevel::try_from(x.security_level)?,
                    requires_subscription: x.requires_subscription,
                    allow_twitch_sub: x.allow_twitch_sub,
                })
            })
            .collect::<Result<Vec<SquadOvCommunity>, SquadOvError>>()?
    )
}

pub async fn edit_community<'a, T>(ex: T, community: &SquadOvCommunity) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        UPDATE squadov.communities
        SET name = $2,
            security_level = $3,
            requires_subscription = $4,
            allow_twitch_sub = $5
        WHERE id = $1
        ",
        community.id,
        &community.name,
        community.security_level as i32,
        community.requires_subscription,
        community.allow_twitch_sub
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn delete_community<'a, T>(ex: T, id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.communities
        WHERE id = $1
        ",
        id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn get_community_from_slug<'a, T>(ex: T, slug: &str) -> Result<SquadOvCommunity, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let x = sqlx::query!(
        "
        SELECT *
        FROM squadov.communities
        WHERE slug = $1
        ",
        slug,
    )
        .fetch_one(ex)
        .await?;
    Ok(
        SquadOvCommunity{
            id: x.id,
            name: x.name,
            slug: x.slug,
            create_tm: x.create_tm,
            creator_user_id: x.creator_user_id,
            security_level: CommunitySecurityLevel::try_from(x.security_level)?,
            requires_subscription: x.requires_subscription,
            allow_twitch_sub: x.allow_twitch_sub,
        }
    )
}

pub async fn get_community_from_id<'a, T>(ex: T, id: i64) -> Result<SquadOvCommunity, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let x = sqlx::query!(
        "
        SELECT *
        FROM squadov.communities
        WHERE id = $1
        ",
        id
    )
        .fetch_one(ex)
        .await?;
    Ok(
        SquadOvCommunity{
            id: x.id,
            name: x.name,
            slug: x.slug,
            create_tm: x.create_tm,
            creator_user_id: x.creator_user_id,
            security_level: CommunitySecurityLevel::try_from(x.security_level)?,
            requires_subscription: x.requires_subscription,
            allow_twitch_sub: x.allow_twitch_sub,
        }
    )
}

pub async fn get_user_community_membership<'a, T>(ex: T, community_id: i64, user_id: i64) -> Result<Option<i64>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            SELECT *
            FROM squadov.community_membership
            WHERE community_id = $1
                AND user_id = $2
            ",
            community_id,
            user_id,
        )
            .fetch_optional(ex)
            .await?
            .map(|x| {
                x.id
            })
    )
}

pub async fn create_commmunity<'a, T>(ex: T, community: &SquadOvCommunity) -> Result<SquadOvCommunity, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let x = sqlx::query!(
        "
        INSERT INTO squadov.communities (
            name,
            create_tm,
            creator_user_id,
            security_level,
            requires_subscription,
            allow_twitch_sub,
            slug
        )
        VALUES (
            $1,
            NOW(),
            $2,
            $3,
            $4,
            $5,
            $6
        )
        RETURNING *
        ",
        community.name,
        community.creator_user_id,
        community.security_level as i32,
        community.requires_subscription,
        community.allow_twitch_sub,
        community.slug,
    )
        .fetch_one(ex)
        .await?;
    Ok(
        SquadOvCommunity{
            id: x.id,
            name: x.name,
            slug: x.slug,
            create_tm: x.create_tm,
            creator_user_id: x.creator_user_id,
            security_level: CommunitySecurityLevel::try_from(x.security_level)?,
            requires_subscription: x.requires_subscription,
            allow_twitch_sub: x.allow_twitch_sub,
        }
    )
}

pub async fn user_join_community<'a, T>(ex: T, community_id: i64, user_id: i64, sub_id: Option<i64>) -> Result<i64, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query!(
            "
            INSERT INTO squadov.community_membership (
                community_id,
                user_id,
                sub_id,
                join_tm
            ) VALUES (
                $1,
                $2,
                $3,
                NOW()
            ) RETURNING id
            ",
            community_id,
            user_id,
            sub_id,
        )
            .fetch_one(ex)
            .await?
            .id
    )
}

pub async fn assign_user_role<'a, T>(ex: T, user_id: i64, role_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        INSERT INTO squadov.community_member_roles (
            role_id,
            membership_id
        )
        SELECT $2, cm.id
        FROM squadov.community_roles AS cr
        INNER JOIN squadov.community_membership AS cm
            ON cm.community_id = cr.community_id
                AND cm.user_id = $1
        WHERE cr.id = $2
        ON CONFLICT DO NOTHING
        ",
        user_id,
        role_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn remove_user_role<'a, T>(ex: T, user_id: i64, role_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.community_member_roles AS cmr
        USING squadov.community_membership AS cm
        WHERE cm.id = cmr.membership_id
            AND cm.user_id = $1
            AND cmr.role_id = $2
        ",
        user_id,
        role_id,
    )
        .execute(ex)
        .await?;
    Ok(())
}

pub async fn kick_user_from_community<'a, T>(ex: T, user_id: i64, community_id: i64) -> Result<(), SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    sqlx::query!(
        "
        DELETE FROM squadov.community_membership
        WHERE user_id = $1
            AND community_id = $2
        ",
        user_id,
        community_id
    )
        .execute(ex)
        .await?;
    Ok(())
}


pub async fn get_user_community_roles<'a, T>(ex: T, community_id: i64, user_id: i64) -> Result<Vec<CommunityRole>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    Ok(
        sqlx::query_as!(
            CommunityRole,
            "
            SELECT cr.*
            FROM squadov.community_roles AS cr
            INNER JOIN squadov.community_member_roles AS cmr
                ON cmr.role_id = cr.id
            INNER JOIN squadov.community_membership AS cm
                ON cm.id = cmr.membership_id
            WHERE cr.community_id = $1
                AND cm.user_id = $2
                AND cm.community_id = $1
            ",
            community_id,
            user_id,
        )
            .fetch_all(ex)
            .await?
    )
}

pub async fn get_users_with_roles_in_community<'a, T>(ex: T, community_id: i64) -> Result<Vec<CommunityUser>, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    let data = sqlx::query!(
        r#"
        SELECT
            u.id,
            u.email,
            u.username,
            u.verified,
            u.uuid,
            u.is_test,
            u.is_admin,
            u.welcome_sent,
            u.registration_time,
            u.support_priority,
            ARRAY_AGG(cr.id) AS "roles!",
            cm.sub_id AS "sub_id"
        FROM squadov.community_membership AS cm
        INNER JOIN squadov.community_member_roles AS cmr
            ON cmr.membership_id = cm.id
        INNER JOIN squadov.community_roles AS cr
            ON cr.id = cmr.role_id
        INNER JOIN squadov.users AS u
            ON u.id = cm.user_id
        WHERE cm.community_id = $1
        GROUP BY u.id, u.email, u.username, u.verified, u.uuid, u.is_test, u.is_admin, u.welcome_sent, u.registration_time, cm.sub_id
        "#,
        community_id,
    )
        .fetch_all(ex)
        .await?;

    Ok(
        data.into_iter().map(|x| {
            CommunityUser{
                user: SquadOVUser{
                    id: x.id,
                    email: x.email,
                    username: x.username,
                    verified: x.verified,
                    uuid: x.uuid,
                    is_test: x.is_test,
                    is_admin: x.is_admin,
                    welcome_sent: x.welcome_sent,
                    registration_time: x.registration_time,
                    support_priority: x.support_priority,
                },
                roles: x.roles,
                sub_id: x.sub_id,
            }
        }).collect()
    )
}