use crate::api;
use squadov_common::stats;
use squadov_common::SquadOvError;
use juniper::FieldResult;

impl api::ApiApplication {
    async fn get_aimlab_linetrace_data(&self, task: &str, mode: i32, user_id: i64, params: &super::GraphqlAimlabStatsParams) -> Result<Vec<stats::AimlabStatLinetraceData>, SquadOvError> {
        let (builder, _) = super::common_aimlab_stat_query_builder("$1", "$2", "$3", params);
        Ok(sqlx::query_as::<_, stats::AimlabStatLinetraceData>(
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

pub struct GraphqlAimlabLinetraceStats {
    pub user_id: i64,
    pub task: String,
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabLinetraceStats {
    async fn ultimate(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatLinetraceData>> {
        Ok(context.app.get_aimlab_linetrace_data(&self.task, 10, self.user_id, &params).await?)
    }
}