use bson::{Document};
use luma_protocol_core::{ProtocolError, Result};
use crate::mql::parser::Filter;

pub struct InsertOp {
    pub db: String,
    pub collection: String,
    pub documents: Vec<Document>,
    pub ordered: bool,
}

pub struct UpdateOp {
    pub db: String,
    pub collection: String,
    pub updates: Vec<UpdateSpec>,
    pub ordered: bool,
}

pub struct UpdateSpec {
    pub q: Document,
    pub u: Document,
    pub upsert: bool,
    pub multi: bool,
}

pub struct DeleteOp {
    pub db: String,
    pub collection: String,
    pub deletes: Vec<DeleteSpec>,
    pub ordered: bool,
}

pub struct DeleteSpec {
    pub q: Document,
    pub limit: i32, // 0 = all, 1 = single
}

pub struct FindOp {
    pub db: String,
    pub collection: String,
    pub filter: Option<Filter>,
    pub skip: i32,
    pub limit: i32,
    pub sort: Option<Document>,
    pub projection: Option<Document>,
}
