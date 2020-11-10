use crate::api;
use squadov_common::stats;
use squadov_common::SquadOvError;
use juniper::FieldResult;

impl api::ApiApplication {
    async fn get_aimlab_decisionshot_data(&self, task: &str, mode: i32, user_uuid: &str, params: &super::GraphqlAimlabStatsParams) -> Result<Vec<stats::AimlabStatDecisionshotData>, SquadOvError> {
        let (builder, _) = super::common_aimlab_stat_query_builder("$1", "$2", "$3::UUID", params);
        Ok(sqlx::query_as::<_, stats::AimlabStatDecisionshotData>(
            &builder
                .select()?
        )
            .bind(task)
            .bind(mode)
            .bind(user_uuid)
            .fetch_all(&*self.pool)
            .await?)
    }
}

pub struct GraphqlAimlabDecisionShotStats {
    pub user_uuid: String,
    pub task: String,
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabDecisionShotStats {
    async fn ultimate(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatDecisionshotData>> {
        Ok(context.app.get_aimlab_decisionshot_data(&self.task, 10, &self.user_uuid, &params).await?)
    }

    async fn precision(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatDecisionshotData>> {
        Ok(context.app.get_aimlab_decisionshot_data(&self.task, 11, &self.user_uuid, &params).await?)
    }

    async fn speed(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatDecisionshotData>> {
        Ok(context.app.get_aimlab_decisionshot_data(&self.task, 12, &self.user_uuid, &params).await?)
    }
}