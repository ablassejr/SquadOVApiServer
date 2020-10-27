mod aimlab;
use crate::common;

#[derive(juniper::GraphQLEnum)]
pub(crate) enum GraphqlStatGroupFunction {
    #[graphql(name="AVG")]
    Avg,
    #[graphql(name="MAX")]
    Max,
    #[graphql(name="MIN")]
    Min
}

pub(crate) fn stat_group_function_to_sql(inner: &str, func: &GraphqlStatGroupFunction) -> String {
    format!(
        "{func}({inner})::DOUBLE PRECISION",
        func=match func {
            GraphqlStatGroupFunction::Avg => "AVG",
            GraphqlStatGroupFunction::Max => "MAX",
            GraphqlStatGroupFunction::Min => "MIN",
        },
        inner=inner
    )
}

#[derive(juniper::GraphQLEnum)]
pub(crate) enum GraphqlSortDirection {
    #[graphql(name="ASC")]
    Asc,
    #[graphql(name="DESC")]
    Desc
}

pub(crate) fn stat_sort_direction_to_sql(d: &GraphqlSortDirection) -> common::SqlOrderDirection {
    match d {
        GraphqlSortDirection::Asc => common::SqlOrderDirection::Asc,
        GraphqlSortDirection::Desc => common::SqlOrderDirection::Desc,
    }
}

pub struct GraphqlAllStats {
    pub user_uuid: String
}

#[juniper::graphql_object(
    Context = super::GraphqlContext,
)]
impl GraphqlAllStats {
    fn aimlab(&self) -> aimlab::GraphqlAimlabStats {
        aimlab::GraphqlAimlabStats{
            user_uuid: self.user_uuid.clone()
        }
    }
}