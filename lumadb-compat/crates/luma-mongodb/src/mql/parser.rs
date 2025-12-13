use bson::{Document, Bson};
use luma_protocol_core::{ProtocolError, Result};

#[derive(Debug, Clone)]
pub enum QueryOperator {
    Eq(Bson),
    Ne(Bson),
    Gt(Bson),
    Gte(Bson),
    Lt(Bson),
    Lte(Bson),
    In(Vec<Bson>),
    Nin(Vec<Bson>),
    And(Vec<Document>),
    Or(Vec<Document>),
    Not(Box<QueryOperator>),
    Nor(Vec<Document>),
    Exists(bool),
    Type(i32), // or String alias
    All(Vec<Bson>),
    ElemMatch(Document),
    Size(i32),
    Regex(String, String),
    Expr(Bson), // Aggregation expression in query
    Text(String),
    Mod(i64, i64),
    Where(String),
    // ...
}

#[derive(Debug, Clone)]
pub struct Filter {
    // Simplified representation: field -> operator(s)
    // Complex logic ($or, $and) handled structurally
    pub clauses: Vec<Clause>,
}

#[derive(Debug, Clone)]
pub enum Clause {
    Field(String, QueryOperator),
    Logical(QueryOperator), // And, Or, Nor
}

pub struct ParsingContext;

impl Filter {
    pub fn parse(doc: &Document) -> Result<Self> {
        let mut clauses = Vec::new();
        for (key, val) in doc {
            if key.starts_with('$') {
                // Top-level logical operator
                let op = parse_operator(key, val)?;
                clauses.push(Clause::Logical(op));
            } else {
                // Field query
                // Value can be:
                // 1. Literal (implicit $eq)
                // 2. Document with operators ({ $gt: 5 })
                if let Bson::Document(d) = val {
                    // check if it's an operator document
                    if d.keys().any(|k| k.starts_with('$')) {
                        for (op_key, op_val) in d {
                             let op = parse_operator(op_key, op_val)?;
                             clauses.push(Clause::Field(key.clone(), op));
                        }
                    } else {
                        // Literal document equality
                        clauses.push(Clause::Field(key.clone(), QueryOperator::Eq(val.clone())));
                    }
                } else {
                    clauses.push(Clause::Field(key.clone(), QueryOperator::Eq(val.clone())));
                }
            }
        }
        Ok(Filter { clauses })
    }
}

fn parse_operator(key: &str, val: &Bson) -> Result<QueryOperator> {
    match key {
        "$eq" => Ok(QueryOperator::Eq(val.clone())),
        "$ne" => Ok(QueryOperator::Ne(val.clone())),
        "$gt" => Ok(QueryOperator::Gt(val.clone())),
        "$gte" => Ok(QueryOperator::Gte(val.clone())),
        "$lt" => Ok(QueryOperator::Lt(val.clone())),
        "$lte" => Ok(QueryOperator::Lte(val.clone())),
        "$in" => {
            if let Bson::Array(arr) = val {
                Ok(QueryOperator::In(arr.clone()))
            } else {
                Err(ProtocolError::Protocol("$in requires array".into()))
            }
        },
        "$nin" => {
            if let Bson::Array(arr) = val {
                Ok(QueryOperator::Nin(arr.clone()))
            } else {
                Err(ProtocolError::Protocol("$nin requires array".into()))
            }
        },
        "$and" => {
             if let Bson::Array(arr) = val {
                 let docs = arr.iter().map(|b| b.as_document().cloned().ok_or(ProtocolError::Protocol("$and requires docs".into()))).collect::<Result<Vec<_>>>()?;
                 Ok(QueryOperator::And(docs))
             } else {
                 Err(ProtocolError::Protocol("$and requires array".into()))
             }
        },
        "$or" => {
             if let Bson::Array(arr) = val {
                 let docs = arr.iter().map(|b| b.as_document().cloned().ok_or(ProtocolError::Protocol("$or requires docs".into()))).collect::<Result<Vec<_>>>()?;
                 Ok(QueryOperator::Or(docs))
             } else {
                 Err(ProtocolError::Protocol("$or requires array".into()))
             }
        },
        // ... Implement other operators as needed ...
        _ => Err(ProtocolError::Protocol(format!("Unknown or unsupported operator: {}", key)))
    }
}
