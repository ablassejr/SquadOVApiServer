mod stats;

use crate::api;
use crate::common;
use actix_web::{web, HttpResponse, HttpRequest};
use juniper::http::GraphQLRequest;
use juniper::http::graphiql::graphiql_source;
use std::sync::Arc;
use juniper::FieldResult;
use uuid::Uuid;

pub struct GraphqlContext {
    app: Arc<api::ApiApplication>,
    session: Option<api::auth::SquadOVSession>
}

impl juniper::Context for GraphqlContext {}

pub struct GraphqlRootQuery {
}

#[juniper::graphql_object(
    Context = GraphqlContext,
)]
impl GraphqlRootQuery {
    async fn stats(context: &GraphqlContext, user_uuid: String) -> FieldResult<stats::GraphqlAllStats> {
        if !context.app.config.server.graphql_debug {
            // Need to verify that the current user has access to this particular user's stats.
            let uuid = Uuid::parse_str(&user_uuid)?;
            let user = match context.app.users.get_stored_user_from_uuid(&uuid, &*context.app.pool).await? {
                Some(x) => x,
                None => return Err(juniper::FieldError::new("Invalid user.", juniper::Value::Null))
            };

            if user.uuid != context.session.as_ref().unwrap().user.uuid {
                return Err(juniper::FieldError::new("No stat access to user.", juniper::Value::Null));
            }
        }

        Ok(stats::GraphqlAllStats{
            user_uuid: user_uuid, 
        })
    }
}

pub type GraphqlSchema = juniper::RootNode<'static, GraphqlRootQuery, juniper::EmptyMutation<GraphqlContext>, juniper::EmptySubscription<GraphqlContext>>;
pub fn create_schema() -> GraphqlSchema {
    GraphqlSchema::new(GraphqlRootQuery{}, juniper::EmptyMutation::new(), juniper::EmptySubscription::new())
}

pub async fn graphql_handler(app : web::Data<Arc<api::ApiApplication>>, data: web::Json<GraphQLRequest>, req: HttpRequest) -> Result<HttpResponse, common::SquadOvError> {
    let context = Arc::new(GraphqlContext{
        app: app.get_ref().clone(),
        session: if !app.config.server.graphql_debug {
            let extensions = req.extensions();
            match extensions.get::<api::auth::SquadOVSession>() {
                Some(s) => Some(s.clone()),
                None => return Err(common::SquadOvError::Unauthorized),
            }
        } else {
            None
        },
    });
    let resp = data.execute(&app.schema, &context).await;
    Ok(HttpResponse::Ok().json(&resp))
}

pub async fn graphiql_handler() -> HttpResponse {
    let html = graphiql_source("http://127.0.0.1:8080/graphql", None);
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}