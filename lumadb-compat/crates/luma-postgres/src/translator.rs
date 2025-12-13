use luma_protocol_core::{
    ir::{Operation, QueryOp, Expr, TableRef, Value, Operator, OrderBy},
    Result, ProtocolError
};
use sqlparser::ast::{
    Statement, Query, SetExpr, Select, TableFactor, Expr as SqlExpr, 
    BinaryOperator, Value as SqlValue, Ident, ObjectName
};

pub struct PostgresTranslator;

impl PostgresTranslator {
    pub fn translate(stmt: Statement) -> Result<Operation> {
        match stmt {
            Statement::Query(query) => {
                let op = Self::translate_query(*query)?;
                Ok(Operation::Query(op))
            }
            _ => Err(ProtocolError::Translator("Unsupported SQL statement".into())),
        }
    }

    fn translate_query(query: Query) -> Result<QueryOp> {
        // Limited support: simple SELECT
        match *query.body {
            SetExpr::Select(select) => Self::translate_select(*select),
            _ => Err(ProtocolError::Translator("Unsupported query body".into())),
        }
    }

    fn translate_select(select: Select) -> Result<QueryOp> {
        // FROM
        let from = if select.from.is_empty() {
            return Err(ProtocolError::Translator("SELECT must have a FROM clause".into()));
        } else {
             Self::translate_table_ref(&select.from[0].relation)?
        };

        // SELECT columns
        let mut select_exprs = Vec::new();
        for item in select.projection {
             select_exprs.push(Self::translate_select_item(item)?);
        }

        // WHERE
        let filter = match select.selection {
            Some(expr) => Some(Self::translate_expr(expr)?),
            None => None,
        };

        Ok(QueryOp {
            select: select_exprs,
            from,
            filter,
            group_by: vec![], // TODO
            order_by: vec![], // TODO
            limit: None,      // TODO
            offset: None,     // TODO
        })
    }

    fn translate_table_ref(table: &TableFactor) -> Result<TableRef> {
        match table {
            TableFactor::Table { name, .. } => {
                let (schema, table_name) = Self::parse_object_name(name)?;
                Ok(TableRef { schema, name: table_name })
            }
            _ => Err(ProtocolError::Translator("Unsupported table factor".into())),
        }
    }

    fn parse_object_name(name: &ObjectName) -> Result<(Option<String>, String)> {
        match name.0.len() {
            1 => Ok((None, name.0[0].value.clone())),
            2 => Ok((Some(name.0[0].value.clone()), name.0[1].value.clone())),
            _ => Err(ProtocolError::Translator("Invalid table name".into())),
        }
    }

    fn translate_select_item(item: sqlparser::ast::SelectItem) -> Result<Expr> {
        match item {
            sqlparser::ast::SelectItem::UnnamedExpr(expr) => Self::translate_expr(expr),
            sqlparser::ast::SelectItem::ExprWithAlias { expr, .. } => Self::translate_expr(expr), // Ignore alias for now
            _ => Err(ProtocolError::Translator("Unsupported select item".into())),
        }
    }

    fn translate_expr(expr: SqlExpr) -> Result<Expr> {
        match expr {
            SqlExpr::Identifier(ident) => Ok(Expr::Column(ident.value)),
            SqlExpr::Value(val) => Self::translate_value(val),
            SqlExpr::BinaryOp { left, op, right } => {
                let l = Box::new(Self::translate_expr(*left)?);
                let r = Box::new(Self::translate_expr(*right)?);
                let operator = Self::translate_binary_op(op)?;
                Ok(Expr::BinaryOp { left: l, op: operator, right: r })
            }
            _ => Err(ProtocolError::Translator("Unsupported expression".into())),
        }
    }

    fn translate_value(val: SqlValue) -> Result<Expr> {
        match val {
            SqlValue::Number(n, _) => {
                // Simplified number parsing
                if let Ok(i) = n.parse::<i64>() {
                    Ok(Expr::Literal(Value::Int(i)))
                } else if let Ok(f) = n.parse::<f64>() {
                    Ok(Expr::Literal(Value::Float(f)))
                } else {
                    Err(ProtocolError::Translator("Invalid number format".into()))
                }
            },
            SqlValue::SingleQuotedString(s) => Ok(Expr::Literal(Value::String(s))),
            SqlValue::Boolean(b) => Ok(Expr::Literal(Value::Bool(b))),
            SqlValue::Null => Ok(Expr::Literal(Value::Null)),
            _ => Err(ProtocolError::Translator("Unsupported value type".into())),
        }
    }

    fn translate_binary_op(op: BinaryOperator) -> Result<Operator> {
        match op {
            BinaryOperator::Eq => Ok(Operator::Eq),
            BinaryOperator::NotEq => Ok(Operator::Ne),
            BinaryOperator::Gt => Ok(Operator::Gt),
            BinaryOperator::Lt => Ok(Operator::Lt),
            BinaryOperator::GtEq => Ok(Operator::Gte),
            BinaryOperator::LtEq => Ok(Operator::Lte),
            BinaryOperator::And => Ok(Operator::And),
            BinaryOperator::Or => Ok(Operator::Or),
            BinaryOperator::Plus => Ok(Operator::Add),
            BinaryOperator::Minus => Ok(Operator::Sub),
            BinaryOperator::Multiply => Ok(Operator::Mul),
            BinaryOperator::Divide => Ok(Operator::Div),
            _ => Err(ProtocolError::Translator("Unsupported operator".into())),
        }
    }
}
