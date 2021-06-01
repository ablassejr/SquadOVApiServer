use crate::SquadOvError;
use uuid::Uuid;
use serde::Serialize;
use sqlx::{Executor, Postgres};
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct MatchVideoSharePermissions {
    pub can_share: bool,
    pub can_clip: bool,
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct LinkShareData {
    pub is_link_shared: bool,
    pub share_url: Option<String>,
}

pub struct ShareEdge {
    id: i64,
    /*
    match_uuid: Option<Uuid>,
    video_uuid: Option<Uuid>,
    source_user_id: i64,
    dest_user_id: Option<i64>,
    dest_squad_id: Option<i64>,
    */
    can_share: bool,
    can_clip: bool,
    parent_connection_id: Option<i64>,
    /*
    share_depth: i32,
    */
    is_terminal: bool,
}

fn trace_edge_permission(start: &ShareEdge, graph: &HashMap<i64, ShareEdge>, current: MatchVideoSharePermissions) -> MatchVideoSharePermissions {
    let start_perm = MatchVideoSharePermissions {
        can_share: start.can_share,
        can_clip: start.can_clip,
    };

    let merge_parent = if let Some(parent_id) = &start.parent_connection_id {
        if let Some(parent_edge) = graph.get(parent_id) {
            trace_edge_permission(parent_edge, graph, start_perm)
        } else {
            start_perm
        }
    } else {
        start_perm
    };

    MatchVideoSharePermissions {
        can_share: merge_parent.can_share && current.can_share,
        can_clip: merge_parent.can_clip && current.can_clip,
    }
}

pub async fn get_match_vod_share_permissions_for_user<'a, T>(ex: T, match_uuid: Option<&Uuid>, video_uuid: Option<&Uuid>, user_id: i64) -> Result<MatchVideoSharePermissions, SquadOvError>
where
    T: Executor<'a, Database = Postgres>
{
    // We're looking for whether or not the user has can_share or can_clip access to the
    // current match/video in question. There are potentially multiple sources of this
    // share access and multiple paths that get to the user in question. To make it simple,
    // as long as there is a single path to the specified user with a permission enabled,
    // then we assume that the user has that permission.
    let edges = sqlx::query_as!(
        ShareEdge,
        r#"
        WITH RECURSIVE access_cte AS (
            SELECT vau.*, TRUE AS "is_terminal"
            FROM squadov.view_share_connections_access_users AS vau
            WHERE ($2::UUID IS NULL OR vau.match_uuid = $2)
                AND ($3::UUID IS NULL OR vau.video_uuid = $3)
                AND vau.user_id = $1
            UNION
            SELECT vau.*, FALSE AS "is_terminal"
            FROM squadov.view_share_connections_access_users AS vau
            INNER JOIN access_cte AS ac
                ON ac.parent_connection_id = vau.id
        )
        SELECT mvc.id, mvc.can_share, mvc.can_clip, mvc.parent_connection_id, at.is_terminal AS "is_terminal!"
        FROM access_cte AS at
        INNER JOIN squadov.share_match_vod_connections AS mvc
            ON mvc.id = at.id
        "#,
        user_id,
        match_uuid,
        video_uuid,
    )
        .fetch_all(ex)
        .await?;

    // First separate out the terminal edges from the non terminal edges.
    // The "terminal" edges are those the edges that connect immediately to the user in question.
    let mut terminal_edges: Vec<ShareEdge> = vec![];
    let mut other_edges: HashMap<i64, ShareEdge> = HashMap::new();
    edges.into_iter().for_each(|x| {
        if x.is_terminal {
            terminal_edges.push(x);
        } else {
            other_edges.insert(x.id, x);
        }
    });

    let mut permission = MatchVideoSharePermissions{
        can_share: false,
        can_clip: false,
    };

    for te in terminal_edges {
        let p = trace_edge_permission(&te, &other_edges, MatchVideoSharePermissions{
            can_share: true,
            can_clip: true,
        });

        permission.can_share |= p.can_share;
        permission.can_clip |= p.can_clip;
    }
    
    Ok(permission)
}