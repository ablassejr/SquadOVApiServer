mod spar;
mod detection;
mod decisionshot;
mod track;
mod erb;
mod linetrace;
mod pentakill;

use crate::api;
use crate::common::sql::{SqlWhere, SqlColumn, SqlJoinTable, SqlTable, SqlQueryBuilder, SqlWhereCondition, SqlWhereCombine};

#[derive(PartialEq)]
#[derive(juniper::GraphQLEnum)]
enum GraphqlAimlabGrouping {
    #[graphql(name="AGID")]
    AgId,
    #[graphql(name="AGDATE")]
    AgDate,
    #[graphql(name="AGDATETIME")]
    AgDateTime,
    #[graphql(name="AGTIME")]
    AgTime,
    #[graphql(name="AGVERSION")]
    AgVersion
}

fn aimlab_grouping_to_select_sql(tbl: &str, group: &GraphqlAimlabGrouping, ref_group: &GraphqlAimlabGrouping) -> String {
    if group == ref_group {
        match group {
            GraphqlAimlabGrouping::AgDate => format!("DATE_TRUNC('day', {}.create_date)", tbl),
            GraphqlAimlabGrouping::AgDateTime => format!("{}.create_date", tbl),
            GraphqlAimlabGrouping::AgTime => format!("{}.create_date::TIME", tbl),
            GraphqlAimlabGrouping::AgVersion => format!("{}.version", tbl),
            GraphqlAimlabGrouping::AgId => format!("ROW_NUMBER() OVER (ORDER BY MAX({}.create_date) ASC)::DOUBLE PRECISION", tbl),
        }
    } else {
        String::from("NULL")
    }
}

fn aimlab_grouping_to_group_column(tbl: &str, group: &GraphqlAimlabGrouping) -> SqlColumn {
    SqlColumn::new_alias(
        &match group {
            GraphqlAimlabGrouping::AgId => format!("{}.id", tbl),
            _ => aimlab_grouping_to_select_sql(tbl, group, group),
        }
    )
}

fn aimlab_grouping_to_order_column(tbl: &str, group: &GraphqlAimlabGrouping) -> SqlColumn {
    SqlColumn::new_alias(
        &match group {
            GraphqlAimlabGrouping::AgId => format!("MAX({}.create_date)", tbl),
            _ => aimlab_grouping_to_select_sql(tbl, group, group),
        }
    )
}

#[derive(juniper::GraphQLInputObject)]
pub(crate) struct GraphqlAimlabStatsParams {
    group: GraphqlAimlabGrouping,
    func: super::GraphqlStatGroupFunction,
    sort: super::GraphqlSortDirection
}

pub(crate) fn common_aimlab_stat_query_builder(task: &str, mode: &str, user_uuid: &str, params: &GraphqlAimlabStatsParams) -> (SqlQueryBuilder, SqlTable) {
    let primary_table = SqlTable{
        name: String::from("squadov.aimlab_tasks"),
        alias: String::from("at"),
    };

    let user_table = SqlTable{
        name: String::from("squadov.users"),
        alias: String::from("users"),
    };

    (SqlQueryBuilder::new(primary_table.clone())
        .join(SqlJoinTable{
            from: (primary_table.clone(), SqlColumn::new_alias("user_id")),
            to: (user_table.clone(), SqlColumn::new_alias("id")),
        }) 
        .column(SqlColumn::new(&super::stat_group_function_to_sql("at.score", &params.func), "score"))
        .column(SqlColumn::new(&aimlab_grouping_to_select_sql("at", &GraphqlAimlabGrouping::AgDate, &params.group), "date"))
        .column(SqlColumn::new(&aimlab_grouping_to_select_sql("at", &GraphqlAimlabGrouping::AgDateTime, &params.group), "datetime"))
        .column(SqlColumn::new(&aimlab_grouping_to_select_sql("at", &GraphqlAimlabGrouping::AgTime, &params.group), "time"))
        .column(SqlColumn::new(&aimlab_grouping_to_select_sql("at", &GraphqlAimlabGrouping::AgVersion, &params.group), "version"))
        .column(SqlColumn::new(&aimlab_grouping_to_select_sql("at", &GraphqlAimlabGrouping::AgId, &params.group), "id"))
        .group(aimlab_grouping_to_group_column("at", &params.group))
        .order(aimlab_grouping_to_order_column("at", &params.group))
        .order_dir(super::stat_sort_direction_to_sql(&params.sort))
        .filter(SqlWhere::new(SqlWhereCondition::Equal(SqlColumn::new_alias("at.task_name"), SqlColumn::new_value(task)), SqlWhereCombine::And))
        .filter(SqlWhere::new(SqlWhereCondition::Equal(SqlColumn::new_alias("at.mode"), SqlColumn::new_value(mode)), SqlWhereCombine::And))
        .filter(SqlWhere::new(SqlWhereCondition::Equal(SqlColumn::new_alias("users.uuid"), SqlColumn::new_value(user_uuid)), SqlWhereCombine::And))
    , primary_table)
}

pub struct GraphqlAimlabStats {
    pub user_uuid: String
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabStats {
    fn gridshot(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("gridshot")
        }
    }

    fn spidershot(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("spidershot")
        }
    }

    fn microshot(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("microshot")
        }
    }

    fn sixshot(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("sixshot")
        }
    }

    fn microflex(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("microflex")
        }
    }

    fn motionshot(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("motionshot")
        }
    }

    fn multishot(&self) -> spar::GraphqlAimlabSparStats {
        spar::GraphqlAimlabSparStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("multishot")
        }
    }

    fn detection(&self) -> detection::GraphqlAimlabDetectionStats {
        detection::GraphqlAimlabDetectionStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("detection")
        }
    }

    fn decisionshot(&self) -> decisionshot::GraphqlAimlabDecisionShotStats {
        decisionshot::GraphqlAimlabDecisionShotStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("decisionshot")
        }
    }

    fn strafetrack(&self) -> track::GraphqlAimlabTrackStats {
        track::GraphqlAimlabTrackStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("strafetrack")
        }
    }

    fn circletrack(&self) -> track::GraphqlAimlabTrackStats {
        track::GraphqlAimlabTrackStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("circletrack")
        }
    }

    fn strafeshot(&self) -> erb::GraphqlAimlabErbStats {
        erb::GraphqlAimlabErbStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("strafeshot")
        }
    }

    fn circleshot(&self) -> erb::GraphqlAimlabErbStats {
        erb::GraphqlAimlabErbStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("circleshot")
        }
    }

    fn linetrace(&self) -> linetrace::GraphqlAimlabLinetraceStats {
        linetrace::GraphqlAimlabLinetraceStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("linetrace")
        }
    }

    fn multilinetrace(&self) -> linetrace::GraphqlAimlabLinetraceStats {
        linetrace::GraphqlAimlabLinetraceStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("multilinetrace")
        }
    }

    fn pentakill(&self) -> pentakill::GraphqlAimlabPentakillStats {
        pentakill::GraphqlAimlabPentakillStats {
            user_uuid: self.user_uuid.clone(),
            task: String::from("pentakill")
        }
    }
}