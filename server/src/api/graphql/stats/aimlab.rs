mod spar;
mod detection;
mod decisionshot;
mod track;
mod erb;
mod linetrace;
mod pentakill;

use crate::api;
use squadov_common::stats::StatPermission;
use squadov_common::sql::{SqlWhere, SqlColumn, SqlJoinTable, SqlTable, SqlQueryBuilder, SqlWhereCondition, SqlWhereCombine};
use juniper::FieldResult;

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

pub(crate) fn common_aimlab_stat_query_builder(task: &str, mode: &str, user_id: &str, params: &GraphqlAimlabStatsParams) -> (SqlQueryBuilder, SqlTable) {
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
        .filter(SqlWhere::new(SqlWhereCondition::Equal(SqlColumn::new_alias("users.id"), SqlColumn::new_value(user_id)), SqlWhereCombine::And))
    , primary_table)
}

pub struct GraphqlAimlabStats {
    pub user_id: i64
}

#[juniper::graphql_object(
    Context = api::graphql::GraphqlContext,
)]
impl GraphqlAimlabStats {
    fn gridshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabGridshot])? {
            return Err(juniper::FieldError::new("No gridshot access.", juniper::Value::Null));
        }

        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("gridshot")
        })
    }

    fn spidershot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabSpidershot])? {
            return Err(juniper::FieldError::new("No spidershot access.", juniper::Value::Null));
        }

        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("spidershot")
        })
    }

    fn microshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabMicroshot])? {
            return Err(juniper::FieldError::new("No microshot access.", juniper::Value::Null));
        }

        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("microshot")
        })
    }

    fn sixshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabSixshot])? {
            return Err(juniper::FieldError::new("No sixshot access.", juniper::Value::Null));
        }

        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("sixshot")
        })
    }

    fn microflex(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabMicroflex])? {
            return Err(juniper::FieldError::new("No microflex access.", juniper::Value::Null));
        }

        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("microflex")
        })
    }

    fn motionshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabMotionshot])? {
            return Err(juniper::FieldError::new("No motionshot access.", juniper::Value::Null));
        }

        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("motionshot")
        })
    }

    fn multishot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<spar::GraphqlAimlabSparStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabMultishot])? {
            return Err(juniper::FieldError::new("No multishot access.", juniper::Value::Null));
        }
        
        Ok(spar::GraphqlAimlabSparStats {
            user_id: self.user_id,
            task: String::from("multishot")
        })
    }

    fn detection(&self, context: &api::graphql::GraphqlContext) -> FieldResult<detection::GraphqlAimlabDetectionStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabDetection])? {
            return Err(juniper::FieldError::new("No detection access.", juniper::Value::Null));
        }

        Ok(detection::GraphqlAimlabDetectionStats {
            user_id: self.user_id,
            task: String::from("detection")
        })
    }

    fn decisionshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<decisionshot::GraphqlAimlabDecisionShotStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabDecisionshot])? {
            return Err(juniper::FieldError::new("No decision shot access.", juniper::Value::Null));
        }

        Ok(decisionshot::GraphqlAimlabDecisionShotStats {
            user_id: self.user_id,
            task: String::from("decisionshot")
        })
    }

    fn strafetrack(&self, context: &api::graphql::GraphqlContext) -> FieldResult<track::GraphqlAimlabTrackStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabStrafetrack])? {
            return Err(juniper::FieldError::new("No strafetrack access.", juniper::Value::Null));
        }

        Ok(track::GraphqlAimlabTrackStats {
            user_id: self.user_id,
            task: String::from("strafetrack")
        })
    }

    fn circletrack(&self, context: &api::graphql::GraphqlContext) -> FieldResult<track::GraphqlAimlabTrackStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabCircletrack])? {
            return Err(juniper::FieldError::new("No circletrack access.", juniper::Value::Null));
        }

        Ok(track::GraphqlAimlabTrackStats {
            user_id: self.user_id,
            task: String::from("circletrack")
        })
    }

    fn strafeshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<erb::GraphqlAimlabErbStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabStrafeshot])? {
            return Err(juniper::FieldError::new("No strafeshot access.", juniper::Value::Null));
        }

        Ok(erb::GraphqlAimlabErbStats {
            user_id: self.user_id,
            task: String::from("strafeshot")
        })
    }

    fn circleshot(&self, context: &api::graphql::GraphqlContext) -> FieldResult<erb::GraphqlAimlabErbStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabCircleshot])? {
            return Err(juniper::FieldError::new("No circleshot access.", juniper::Value::Null));
        }

        Ok(erb::GraphqlAimlabErbStats {
            user_id: self.user_id,
            task: String::from("circleshot")
        })
    }

    fn linetrace(&self, context: &api::graphql::GraphqlContext) -> FieldResult<linetrace::GraphqlAimlabLinetraceStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabLinetrace])? {
            return Err(juniper::FieldError::new("No line trace access.", juniper::Value::Null));
        }

        Ok(linetrace::GraphqlAimlabLinetraceStats {
            user_id: self.user_id,
            task: String::from("linetrace")
        })
    }

    fn multilinetrace(&self, context: &api::graphql::GraphqlContext) -> FieldResult<linetrace::GraphqlAimlabLinetraceStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabMultilinetrace])? {
            return Err(juniper::FieldError::new("No multilinetrace access.", juniper::Value::Null));
        }

        Ok(linetrace::GraphqlAimlabLinetraceStats {
            user_id: self.user_id,
            task: String::from("multilinetrace")
        })
    }

    fn pentakill(&self, context: &api::graphql::GraphqlContext) -> FieldResult<pentakill::GraphqlAimlabPentakillStats> {
        if !context.has_access_to_stat(&[StatPermission::AimlabPentakill])? {
            return Err(juniper::FieldError::new("No gridshot access.", juniper::Value::Null));
        }
        
        Ok(pentakill::GraphqlAimlabPentakillStats {
            user_id: self.user_id,
            task: String::from("pentakill")
        })
    }
}