use crate::api;
use crate::common::stats;
use crate::common::SquadOvError;
use juniper::FieldResult;

impl api::ApiApplication {
    async fn get_aimlab_linetrace_data(&self, task: &str, mode: i32, user_uuid: &str, params: &super::GraphqlAimlabStatsParams) -> Result<Vec<stats::AimlabStatLinetraceData>, SquadOvError> {
        let (builder, _) = super::common_aimlab_stat_query_builder("$1", "$2", "$3::UUID", params);
        Ok(sqlx::query_as::<_, stats::AimlabStatLinetraceData>(
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

pub struct GraphqlAimlabLinetraceStats {
    pub user_uuid: String,
    pub task: String,
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabLinetraceStats {
    async fn ultimate(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatLinetraceData>> {
        Ok(context.app.get_aimlab_linetrace_data(&self.task, 10, &self.user_uuid, &params).await?)
    }
}