
/// Fluent query builder for constructing SQL queries
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    select: Vec<String>,
    from: Option<String>,
    joins: Vec<Join>,
    where_clauses: Vec<WhereClause>,
    order_by: Vec<OrderBy>,
    group_by: Vec<String>,
    having: Vec<WhereClause>,
    limit: Option<u64>,
    offset: Option<u64>,
    query_type: QueryType,
    insert_table: Option<String>,
    insert_values: Vec<Vec<String>>,
    update_table: Option<String>,
    update_values: Vec<(String, String)>,
    delete_table: Option<String>,
}

#[derive(Debug, Clone)]
enum QueryType {
    Select,
    Insert,
    Update,
    Delete,
}

#[derive(Debug, Clone)]
struct Join {
    join_type: JoinType,
    table: String,
    condition: String,
}

#[derive(Debug, Clone)]
enum JoinType {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Debug, Clone)]
struct WhereClause {
    column: String,
    operator: String,
    value: String,
    logical_op: LogicalOp,
}

#[derive(Debug, Clone)]
enum LogicalOp {
    And,
    Or,
}

#[derive(Debug, Clone)]
struct OrderBy {
    column: String,
    direction: OrderDirection,
}

#[derive(Debug, Clone)]
enum OrderDirection {
    Asc,
    Desc,
}

impl QueryBuilder {
    /// Create a new SELECT query builder
    pub fn select(columns: &[&str]) -> Self {
        Self {
            select: columns.iter().map(|s| s.to_string()).collect(),
            from: None,
            joins: Vec::new(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            query_type: QueryType::Select,
            insert_table: None,
            insert_values: Vec::new(),
            update_table: None,
            update_values: Vec::new(),
            delete_table: None,
        }
    }

    /// Create a new SELECT * query builder
    pub fn select_all() -> Self {
        Self::select(&["*"])
    }

    /// Set the FROM clause
    pub fn from(mut self, table: &str) -> Self {
        self.from = Some(table.to_string());
        self
    }

    /// Add an INNER JOIN
    pub fn inner_join(mut self, table: &str, condition: &str) -> Self {
        self.joins.push(Join {
            join_type: JoinType::Inner,
            table: table.to_string(),
            condition: condition.to_string(),
        });
        self
    }

    /// Add a LEFT JOIN
    pub fn left_join(mut self, table: &str, condition: &str) -> Self {
        self.joins.push(Join {
            join_type: JoinType::Left,
            table: table.to_string(),
            condition: condition.to_string(),
        });
        self
    }

    /// Add a WHERE clause with AND
    pub fn where_eq(mut self, column: &str, value: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: "=".to_string(),
            value: format!("'{}'", value.replace("'", "''")),
            logical_op: LogicalOp::And,
        });
        self
    }

    /// Add a WHERE clause with AND (numeric)
    pub fn where_eq_num(mut self, column: &str, value: i64) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: "=".to_string(),
            value: value.to_string(),
            logical_op: LogicalOp::And,
        });
        self
    }

    /// Add a WHERE clause with AND (parameterized)
    pub fn where_eq_param(mut self, column: &str, param: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: "=".to_string(),
            value: param.to_string(),
            logical_op: LogicalOp::And,
        });
        self
    }

    /// Add a WHERE clause with OR
    pub fn or_where_eq(mut self, column: &str, value: &str) -> Self {
        self.where_clauses.push(WhereClause {
            column: column.to_string(),
            operator: "=".to_string(),
            value: format!("'{}'", value.replace("'", "''")),
            logical_op: LogicalOp::Or,
        });
        self
    }

    /// Add ORDER BY clause
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by.push(OrderBy {
            column: column.to_string(),
            direction: if direction.to_uppercase() == "DESC" {
                OrderDirection::Desc
            } else {
                OrderDirection::Asc
            },
        });
        self
    }

    /// Add LIMIT clause
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add OFFSET clause
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Create a new INSERT query builder
    pub fn insert(table: &str) -> Self {
        Self {
            select: Vec::new(),
            from: None,
            joins: Vec::new(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            query_type: QueryType::Insert,
            insert_table: Some(table.to_string()),
            insert_values: Vec::new(),
            update_table: None,
            update_values: Vec::new(),
            delete_table: None,
        }
    }

    /// Add values to INSERT
    pub fn values(mut self, values: Vec<&str>) -> Self {
        self.insert_values.push(values.iter().map(|s| {
            if s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok() {
                s.to_string()
            } else {
                format!("'{}'", s.replace("'", "''"))
            }
        }).collect());
        self
    }

    /// Create a new UPDATE query builder
    pub fn update(table: &str) -> Self {
        Self {
            select: Vec::new(),
            from: None,
            joins: Vec::new(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            query_type: QueryType::Update,
            insert_table: None,
            insert_values: Vec::new(),
            update_table: Some(table.to_string()),
            update_values: Vec::new(),
            delete_table: None,
        }
    }

    /// Set a column value in UPDATE
    pub fn set(mut self, column: &str, value: &str) -> Self {
        let val = if value.parse::<i64>().is_ok() || value.parse::<f64>().is_ok() {
            value.to_string()
        } else {
            format!("'{}'", value.replace("'", "''"))
        };
        self.update_values.push((column.to_string(), val));
        self
    }

    /// Create a new DELETE query builder
    pub fn delete(table: &str) -> Self {
        Self {
            select: Vec::new(),
            from: None,
            joins: Vec::new(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit: None,
            offset: None,
            query_type: QueryType::Delete,
            insert_table: None,
            insert_values: Vec::new(),
            update_table: None,
            update_values: Vec::new(),
            delete_table: Some(table.to_string()),
        }
    }

    /// Convert the query builder to SQL string
    pub fn to_sql(&self) -> String {
        match &self.query_type {
            QueryType::Select => self.build_select(),
            QueryType::Insert => self.build_insert(),
            QueryType::Update => self.build_update(),
            QueryType::Delete => self.build_delete(),
        }
    }

    fn build_select(&self) -> String {
        let mut sql = String::new();
        
        sql.push_str("SELECT ");
        sql.push_str(&self.select.join(", "));
        
        if let Some(ref from) = self.from {
            sql.push_str(" FROM ");
            sql.push_str(from);
        }
        
        for join in &self.joins {
            match join.join_type {
                JoinType::Inner => sql.push_str(" INNER JOIN "),
                JoinType::Left => sql.push_str(" LEFT JOIN "),
                JoinType::Right => sql.push_str(" RIGHT JOIN "),
                JoinType::Full => sql.push_str(" FULL JOIN "),
            }
            sql.push_str(&join.table);
            sql.push_str(" ON ");
            sql.push_str(&join.condition);
        }
        
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            for (i, clause) in self.where_clauses.iter().enumerate() {
                if i > 0 {
                    match clause.logical_op {
                        LogicalOp::And => sql.push_str(" AND "),
                        LogicalOp::Or => sql.push_str(" OR "),
                    }
                }
                sql.push_str(&clause.column);
                sql.push_str(" ");
                sql.push_str(&clause.operator);
                sql.push_str(" ");
                sql.push_str(&clause.value);
            }
        }
        
        if !self.order_by.is_empty() {
            sql.push_str(" ORDER BY ");
            let orders: Vec<String> = self.order_by.iter().map(|ob| {
                format!(
                    "{} {}",
                    ob.column,
                    match ob.direction {
                        OrderDirection::Asc => "ASC",
                        OrderDirection::Desc => "DESC",
                    }
                )
            }).collect();
            sql.push_str(&orders.join(", "));
        }
        
        if let Some(limit) = self.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = self.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }
        
        sql
    }

    fn build_insert(&self) -> String {
        let mut sql = String::new();
        
        if let Some(ref table) = self.insert_table {
            sql.push_str("INSERT INTO ");
            sql.push_str(table);
            
            if !self.insert_values.is_empty() {
                sql.push_str(" VALUES ");
                let values: Vec<String> = self.insert_values.iter().map(|row| {
                    format!("({})", row.join(", "))
                }).collect();
                sql.push_str(&values.join(", "));
            }
        }
        
        sql
    }

    fn build_update(&self) -> String {
        let mut sql = String::new();
        
        if let Some(ref table) = self.update_table {
            sql.push_str("UPDATE ");
            sql.push_str(table);
            sql.push_str(" SET ");
            
            let sets: Vec<String> = self.update_values.iter()
                .map(|(col, val)| format!("{} = {}", col, val))
                .collect();
            sql.push_str(&sets.join(", "));
            
            if !self.where_clauses.is_empty() {
                sql.push_str(" WHERE ");
                for (i, clause) in self.where_clauses.iter().enumerate() {
                    if i > 0 {
                        match clause.logical_op {
                            LogicalOp::And => sql.push_str(" AND "),
                            LogicalOp::Or => sql.push_str(" OR "),
                        }
                    }
                    sql.push_str(&clause.column);
                    sql.push_str(" ");
                    sql.push_str(&clause.operator);
                    sql.push_str(" ");
                    sql.push_str(&clause.value);
                }
            }
        }
        
        sql
    }

    fn build_delete(&self) -> String {
        let mut sql = String::new();
        
        if let Some(ref table) = self.delete_table {
            sql.push_str("DELETE FROM ");
            sql.push_str(table);
            
            if !self.where_clauses.is_empty() {
                sql.push_str(" WHERE ");
                for (i, clause) in self.where_clauses.iter().enumerate() {
                    if i > 0 {
                        match clause.logical_op {
                            LogicalOp::And => sql.push_str(" AND "),
                            LogicalOp::Or => sql.push_str(" OR "),
                        }
                    }
                    sql.push_str(&clause.column);
                    sql.push_str(" ");
                    sql.push_str(&clause.operator);
                    sql.push_str(" ");
                    sql.push_str(&clause.value);
                }
            }
        }
        
        sql
    }
}

