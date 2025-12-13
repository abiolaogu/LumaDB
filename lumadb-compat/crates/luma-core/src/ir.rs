use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Query(QueryOp),
    DML(DmlOp),
    DDL(DdlOp),
    Command(CommandOp),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryOp {
    pub select: Vec<Expr>,
    pub from: TableRef,
    pub filter: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub order_by: Vec<OrderBy>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DmlOp {
    Insert { table: TableRef, columns: Vec<String>, values: Vec<Vec<Value>> },
    Update { table: TableRef, assignments: HashMap<String, Expr>, filter: Option<Expr> },
    Delete { table: TableRef, filter: Option<Expr> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DdlOp {
    CreateTable { name: TableRef, columns: Vec<ColumnDef> },
    DropTable { name: TableRef },
    AlterTable { name: TableRef, operation: AlterOp },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandOp {
    Handshake(String), // Protocol specific handshake info
    Ping,
    SetOption(String, Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRef {
    pub schema: Option<String>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    Column(String),
    Literal(Value),
    BinaryOp { left: Box<Expr>, op: Operator, right: Box<Expr> },
    UnaryOp { op: UnaryOperator, expr: Box<Expr> },
    FunctionCall { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Timestamp(i64),
    Uuid(Vec<u8>),
    List(Vec<Value>),
    // Removed Map temporarily to simplify recursion debug if it's the culprit
    // Map(HashMap<String, Value>), 
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operator {
    Eq, Ne, Gt, Lt, Gte, Lte,
    And, Or, Like,
    Add, Sub, Mul, Div,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not, IsNull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBy {
    pub expr: Expr,
    pub asc: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataType {
    Int, Float, String, Boolean, Timestamp, Bytes, UUID, Map, List
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlterOp {
    AddColumn(ColumnDef),
    DropColumn(String),
}
