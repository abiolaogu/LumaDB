use super::ir::*;
use crate::{Result, LumaError, Document, types::Value};
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;
use sqlparser::ast::{Statement, SetExpr, Expr, Value as SqlValue};
use std::collections::HashMap;

pub struct QueryParser;

impl QueryParser {
    /// Parse SQL to IR
    pub fn parse_sql(sql: &str) -> Result<QueryPlan> {
        let dialect = GenericDialect {};
        let ast = Parser::parse_sql(&dialect, sql)
            .map_err(|e| LumaError::Internal(format!("SQL Parse Error: {}", e)))?;

        // Handle first statement only for now
        if let Some(stmt) = ast.into_iter().next() {
            match stmt {
                Statement::Query(query) => {
                    if let SetExpr::Select(select) = *query.body {
                        let from = select.from;
                        if from.is_empty() { return Ok(QueryPlan::Ping); } // Approximation
                        
                        let collection = format!("{}", from[0].relation);
                        // Extract projection
                        let projection = if select.projection.iter().any(|item| matches!(item, sqlparser::ast::SelectItem::Wildcard(_))) {
                            None
                        } else {
                            let cols: Vec<String> = select.projection.iter().filter_map(|item| {
                                match item {
                                    sqlparser::ast::SelectItem::UnnamedExpr(Expr::Identifier(ident)) => Some(ident.value.clone()),
                                    sqlparser::ast::SelectItem::ExprWithAlias { alias, .. } => Some(alias.value.clone()), // Support alias as column name
                                    _ => None,
                                }
                            }).collect();
                            
                            if cols.is_empty() { None } else { Some(cols) }
                        };

                        Ok(QueryPlan::Select(SelectPlan {
                            collection,
                            filter: None,
                            projection,
                            limit: None,
                        }))
                    } else {
                        Err(LumaError::Internal("Unsupported query type".into()))
                    }
                },
                Statement::Insert { table_name, columns, source, .. } => {
                     let collection = format!("{}", table_name);
                     let mut docs = Vec::new();
                     
                     if let Some(query) = source {
                        if let SetExpr::Values(values) = *query.body {
                            for row in values.rows {
                                let mut doc_map = HashMap::new();
                                for (i, expr) in row.into_iter().enumerate() {
                                    let col_name = columns.get(i).map(|i| i.value.clone()).unwrap_or(format!("col_{}", i));
                                    let val = match expr {
                                        Expr::Value(SqlValue::Number(n, _)) => n.parse::<f64>().unwrap_or(0.0).into(),
                                        Expr::Value(SqlValue::SingleQuotedString(s)) => s.into(),
                                        _ => "unsupported".to_string().into(),
                                    };
                                    doc_map.insert(col_name, val);
                                }
                                docs.push(Document::new(doc_map));
                            }
                        }
                    }
                    Ok(QueryPlan::Insert(InsertPlan { collection, documents: docs }))
                },
                _ => Err(LumaError::Internal("Unsupported SQL statement".into())),
            }
        } else {
             Err(LumaError::Internal("Empty query".into()))
        }
    }

    /// Parse Mongo Command to IR
    pub fn parse_mongo(doc: bson::Document) -> Result<QueryPlan> {
        if let Ok(coll_name) = doc.get_str("insert") {
             if let Ok(docs_arr) = doc.get_array("documents") {
                 let mut docs = Vec::new();
                 for d in docs_arr {
                     if let bson::Bson::Document(d_doc) = d {
                         let mut map = HashMap::new();
                         for (k, v) in d_doc {
                             // Conversion logic simplified
                             let val: Value = match v {
                                 bson::Bson::String(s) => s.clone().into(),
                                 bson::Bson::Int32(i) => (*i as i64).into(),
                                 bson::Bson::Int64(i) => (*i).into(),
                                 bson::Bson::Double(f) => (*f).into(),
                                 _ => v.to_string().into(),
                             };
                             map.insert(k.clone(), val);
                         }
                         docs.push(Document::new(map));
                     }
                 }
                 return Ok(QueryPlan::Insert(InsertPlan { 
                     collection: coll_name.to_string(), 
                     documents: docs 
                 }));
             }
        }
        
        if let Ok(coll_name) = doc.get_str("find") {
            Ok(QueryPlan::Select(SelectPlan {
                collection: coll_name.to_string(),
                filter: None, // TODO
                projection: None,
                limit: None,
            }))
        } else {
            // Check for hello/isMaster... actually that's handled in adapter usually?
            // If adapter passes it here, we return Ping or similar.
            Ok(QueryPlan::Ping)
        }
    }
}
