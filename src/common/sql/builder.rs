use crate::common::SquadOvError;

pub enum SqlOrderDirection {
    Asc,
    Desc
}

#[derive(Clone)]
pub struct SqlTable {
    pub name: String,
    pub alias: String,
}

impl SqlTable {
    fn to_sql_from(&self) -> String {
        format!("FROM {} AS \"{}\"", &self.name, &self.alias)
    }
}

#[derive(Clone)]
pub struct SqlColumn {
    pub value: Option<String>,
    pub alias: Option<String>
}

impl SqlColumn {
    pub fn new(v: &str, a: &str) -> Self {
        SqlColumn {
            value: Some(String::from(v)),
            alias: Some(String::from(a)),
        }
    }

    pub fn new_alias(v: &str) -> Self {
        SqlColumn {
            value: None,
            alias: Some(String::from(v)),
        }
    }

    pub fn new_value(v: &str) -> Self {
        SqlColumn {
            value: Some(String::from(v)),
            alias: None,
        }
    }

    fn value_or_alias(&self) -> Result<String, SquadOvError> {
        if self.value.is_some() {
            Ok(self.value.as_ref().unwrap().clone())
        } else if self.alias.is_some() {
            Ok(self.alias.as_ref().unwrap().clone())
        } else {
            Err(SquadOvError::BadRequest)
        }
    }
}

pub enum SqlWhereCondition {
    Equal(SqlColumn, SqlColumn)
}

pub enum SqlWhereCombine {
    And,
//    Or
}

pub struct SqlWhere {
    condition: SqlWhereCondition,
    combine: SqlWhereCombine
}

impl SqlWhere {
    pub fn new(cond: SqlWhereCondition, comb: SqlWhereCombine) -> Self {
        Self {
            condition: cond,
            combine: comb,
        }
    }

    fn condition_to_sql(&self) -> Result<String, SquadOvError> {
        match &self.condition {
            SqlWhereCondition::Equal(c1, c2) => Ok(format!("{} = {}", c1.value_or_alias()?, c2.value_or_alias()?))
        }
    }

    fn combine_to_sql(&self) -> String {
        String::from(
            match self.combine {
                SqlWhereCombine::And => "AND",
//                SqlWhereCombine::Or => "OR"
            }
        )
    }
} 

#[derive(Clone)]
pub struct SqlJoinTable {
    pub from: (SqlTable, SqlColumn),
    pub to: (SqlTable, SqlColumn),
}

impl SqlJoinTable {
    fn to_sql_inner(&self) -> Result<String, SquadOvError> {
        if self.from.1.alias.is_none() || self.to.1.alias.is_none() {
            return Err(SquadOvError::BadRequest);
        }

        Ok(format!(r#"
        INNER JOIN {to} AS "{alias}" ON {from}.{from_col} = {alias}.{to_col}
        "#,
            to=self.to.0.name,
            alias=self.to.0.alias,
            from=self.from.0.alias,
            from_col=self.from.1.alias.as_ref().unwrap(),
            to_col=self.to.1.alias.as_ref().unwrap(),
        ))
    }
}

pub struct SqlQueryBuilder {
    primary_table: SqlTable,
    joined_tables: Vec<SqlJoinTable>,
    columns: Vec<SqlColumn>,
    group_columns: Vec<SqlColumn>,
    order_columns: Vec<SqlColumn>,
    order_direction: SqlOrderDirection,
    filters: Vec<SqlWhere>
}

impl SqlQueryBuilder {
    pub fn new(primary: SqlTable) -> Self {
        SqlQueryBuilder {
            primary_table: primary,
            joined_tables: Vec::new(),
            columns: Vec::new(),
            group_columns: Vec::new(),
            order_columns: Vec::new(),
            order_direction: SqlOrderDirection::Asc,
            filters: Vec::new(),
        }
    }

    pub fn column(mut self, col: SqlColumn) -> Self {
        self.columns.push(col);
        self
    }

    pub fn join(mut self, join: SqlJoinTable) -> Self {
        self.joined_tables.push(join);
        self
    }

    pub fn group(mut self, col: SqlColumn) -> Self {
        self.group_columns.push(col);
        self
    }

    pub fn order(mut self, col: SqlColumn) -> Self {
        self.order_columns.push(col);
        self
    }

    pub fn order_dir(mut self, dir: SqlOrderDirection) -> Self {
        self.order_direction = dir;
        self
    }

    pub fn filter(mut self, s : SqlWhere) -> Self {
        self.filters.push(s);
        self
    }

    fn gen_filter_clause(&self) -> Result<String, SquadOvError> {
        let mut sql: Vec<String> = Vec::new();
        sql.push(String::from("WHERE"));
        for (idx, f) in self.filters.iter().enumerate() {
            if idx > 0 {
                sql.push(f.combine_to_sql());
            }
            sql.push(f.condition_to_sql()?);
        }

        Ok(sql.join(" "))
    }

    fn gen_group_clause(&self) -> Result<String, SquadOvError> {
        let mut groups: Vec<String> = Vec::new();
        for g in &self.group_columns {
            if g.alias.is_none() {
                return Err(SquadOvError::BadRequest)
            }
            groups.push(g.alias.as_ref().unwrap().clone());
        }
        Ok(format!("GROUP BY {}", groups.join(",")))
    }

    fn gen_order_clause(&self) -> Result<String, SquadOvError> {
        let mut order_by: Vec<String> = Vec::new();
        for o in &self.order_columns {
            if o.alias.is_none() {
                return Err(SquadOvError::BadRequest)
            }
            order_by.push(o.alias.as_ref().unwrap().clone());
        }

        Ok(format!(
            "ORDER BY {by} {dir}",
            by=order_by.join(","),
            dir=match self.order_direction {
                SqlOrderDirection::Asc => "ASC",
                SqlOrderDirection::Desc => "DESC",
            }
        ))
    }

    pub fn select(self) -> Result<String, SquadOvError> {
        let mut sql : Vec<String> = Vec::new();
        sql.push(String::from("SELECT"));
        for c in &self.columns {
            if c.value.is_none() || c.alias.is_none() {
                return Err(SquadOvError::BadRequest);
            }
            sql.push(format!(r#"{} as "{}""#, c.value.as_ref().unwrap(), c.alias.as_ref().unwrap()));
            sql.push(String::from(","));
        }
        sql.truncate(sql.len() - 1);

        sql.push(self.primary_table.to_sql_from());
        for j in &self.joined_tables {
            sql.push(j.to_sql_inner()?);
        }

        sql.push(self.gen_filter_clause()?);
        sql.push(self.gen_group_clause()?);
        sql.push(self.gen_order_clause()?);
        Ok(sql.join(" "))
    }
}