use crate::api;
use crate::api::graphql;
use squadov_common::stats;
use squadov_common::SquadOvError;
use squadov_common::sql::{SqlColumn, SqlJoinTable, SqlTable};
use juniper::FieldResult;

impl api::ApiApplication {
    async fn get_aimlab_spar_data(&self, task: &str, mode: i32, user_uuid: &str, params: &super::GraphqlAimlabStatsParams) -> Result<Vec<stats::AimlabStatSparData>, SquadOvError> {
        let secondary_table = SqlTable{
            name: String::from("squadov.view_aimlab_spar_data"),
            alias: String::from("vasd"),
        };

        let (builder, primary_table) = super::common_aimlab_stat_query_builder("$1", "$2", "$3::UUID", params);
        Ok(sqlx::query_as::<_, stats::AimlabStatSparData>(
            &builder
                .join(SqlJoinTable{
                    from: (primary_table.clone(), SqlColumn::new_alias("match_uuid")),
                    to: (secondary_table.clone(), SqlColumn::new_alias("match_uuid")),
                }) 
                .column(SqlColumn::new(&graphql::stats::stat_group_function_to_sql("vasd.kill", &params.func), "kill"))
                .column(SqlColumn::new(&graphql::stats::stat_group_function_to_sql("vasd.ttk", &params.func), "ttk"))
                .column(SqlColumn::new(&graphql::stats::stat_group_function_to_sql("vasd.acc", &params.func), "acc"))
                .select()?
        )
            .bind(task)
            .bind(mode)
            .bind(user_uuid)
            .fetch_all(&*self.pool)
            .await?)
    }
}

pub struct GraphqlAimlabSparStats {
    pub user_uuid: String,
    pub task: String,
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabSparStats {
    async fn ultimate(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatSparData>> {
        Ok(context.app.get_aimlab_spar_data(&self.task, 10, &self.user_uuid, &params).await?)
    }

    async fn precision(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatSparData>> {
        Ok(context.app.get_aimlab_spar_data(&self.task, 11, &self.user_uuid, &params).await?)
    }

    async fn speed(&self, context: &api::graphql::GraphqlContext, params: super::GraphqlAimlabStatsParams) -> FieldResult<Vec<stats::AimlabStatSparData>> {
        Ok(context.app.get_aimlab_spar_data(&self.task, 12, &self.user_uuid, &params).await?)
    }
}