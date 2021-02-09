use crate::api;
use squadov_common::stats;
use squadov_common::SquadOvError;
use juniper::FieldResult;

impl api::ApiApplication {
    async fn get_aimlab_erb_data(&self, task: &str, mode: i32, user_id: i64, params: &super::GraphqlAimlabStatsParams) -> Result<Vec<stats::AimlabStatErbData>, SquadOvError> {
        let (builder, _) = super::common_aimlab_stat_query_builder("$1", "$2", "$3", params);
        Ok(sqlx::query_as::<_, stats::AimlabStatErbData>(
            &builder
                .select()?
        )
            .bind(task)
            .bind(mode)
            .bind(user_id)
            .fetch_all(&*self.pool)
            .await?)
    }
}

pub struct GraphqlAimlabErbStats {
    pub user_id: i64,
    pub task: String,
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabErbStats {
    async fn ultimate(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatErbData>> {
        Ok(context.app.get_aimlab_erb_data(&self.task, 10, self.user_id, &params).await?)
    }
}