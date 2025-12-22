//! TDengine SQL Parser Structures
//!
//! Type definitions for TDengine SQL parsing.

use std::collections::HashMap;

/// Window clause types
#[derive(Debug, Clone)]
pub enum WindowClause {
    /// INTERVAL(duration) [SLIDING(duration)] [FILL(...)]
    Interval {
        interval: String,
        offset: Option<String>,
        sliding: Option<String>,
        fill: FillClause,
    },
    /// SESSION(ts_col, tolerance)
    Session {
        ts_column: String,
        tolerance: String,
    },
    /// STATE_WINDOW(column)
    State {
        column: String,
    },
    /// EVENT_WINDOW START WITH condition END WITH condition
    Event {
        start_condition: Expr,
        end_condition: Expr,
    },
    /// COUNT_WINDOW(count [, sliding])
    Count {
        count: i64,
        sliding: Option<i64>,
    },
}

/// Fill clause for INTERVAL windows
#[derive(Debug, Clone, PartialEq)]
pub enum FillClause {
    /// No fill - skip empty windows
    None,
    /// Fill with NULL
    Null,
    /// Fill with NULL (alias)
    NullF,
    /// Fill with specific values
    Value(Vec<f64>),
    /// Fill with previous value
    Prev,
    /// Fill with next value
    Next,
    /// Linear interpolation
    Linear,
    /// Nearest value
    Nearest,
}

impl Default for FillClause {
    fn default() -> Self {
        FillClause::None
    }
}

/// Expression types for conditions
#[derive(Debug, Clone)]
pub enum Expr {
    /// Column reference
    Column(String),
    /// Literal value
    Literal(LiteralValue),
    /// Binary operation
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOperator,
        right: Box<Expr>,
    },
    /// Unary operation
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expr>,
    },
    /// Function call
    Function {
        name: String,
        args: Vec<Expr>,
    },
    /// Subquery
    Subquery(Box<SelectStatement>),
    /// BETWEEN expression
    Between {
        expr: Box<Expr>,
        low: Box<Expr>,
        high: Box<Expr>,
        negated: bool,
    },
    /// IN expression
    In {
        expr: Box<Expr>,
        list: Vec<Expr>,
        negated: bool,
    },
    /// LIKE expression
    Like {
        expr: Box<Expr>,
        pattern: String,
        negated: bool,
    },
    /// IS NULL expression
    IsNull {
        expr: Box<Expr>,
        negated: bool,
    },
    /// CASE expression
    Case {
        operand: Option<Box<Expr>>,
        when_clauses: Vec<(Expr, Expr)>,
        else_clause: Option<Box<Expr>>,
    },
    /// Wildcard (*)
    Wildcard,
    /// Qualified wildcard (table.*)
    QualifiedWildcard(String),
}

/// Literal values
#[derive(Debug, Clone)]
pub enum LiteralValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Timestamp(i64),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    
    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    
    // Logical
    And,
    Or,
    
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Not,
    Minus,
    Plus,
    BitwiseNot,
}

/// SELECT statement structure
#[derive(Debug, Clone)]
pub struct SelectStatement {
    pub distinct: bool,
    pub columns: Vec<SelectItem>,
    pub from: Option<FromClause>,
    pub where_clause: Option<Expr>,
    pub group_by: Vec<Expr>,
    pub having: Option<Expr>,
    pub window: Option<WindowClause>,
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub slimit: Option<i64>,  // TDengine-specific: limit for number of tables
    pub soffset: Option<i64>, // TDengine-specific: offset for tables
}

/// SELECT item (column or expression with optional alias)
#[derive(Debug, Clone)]
pub struct SelectItem {
    pub expr: Expr,
    pub alias: Option<String>,
}

/// FROM clause
#[derive(Debug, Clone)]
pub enum FromClause {
    Table {
        name: String,
        database: Option<String>,
        alias: Option<String>,
    },
    SubQuery {
        query: Box<SelectStatement>,
        alias: String,
    },
    Join {
        left: Box<FromClause>,
        right: Box<FromClause>,
        join_type: JoinType,
        condition: Option<Expr>,
    },
}

/// JOIN types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// ORDER BY item
#[derive(Debug, Clone)]
pub struct OrderByItem {
    pub expr: Expr,
    pub asc: bool,
    pub nulls_first: Option<bool>,
}

/// INSERT statement structure
#[derive(Debug, Clone)]
pub struct InsertStatement {
    pub table: TableRef,
    pub using: Option<UsingClause>,
    pub columns: Option<Vec<String>>,
    pub values: Vec<Vec<Expr>>,
}

/// Table reference with optional database
#[derive(Debug, Clone)]
pub struct TableRef {
    pub database: Option<String>,
    pub table: String,
}

/// USING clause for auto-create subtables
#[derive(Debug, Clone)]
pub struct UsingClause {
    pub stable: TableRef,
    pub tags: Vec<Expr>,
}

/// CREATE DATABASE statement
#[derive(Debug, Clone)]
pub struct CreateDatabaseStatement {
    pub if_not_exists: bool,
    pub name: String,
    pub options: DatabaseOptions,
}

/// Database options
#[derive(Debug, Clone, Default)]
pub struct DatabaseOptions {
    pub precision: Option<String>,
    pub replica: Option<i32>,
    pub keep: Option<String>,
    pub buffer: Option<i32>,
    pub pages: Option<i32>,
    pub page_size: Option<i32>,
    pub wal: Option<i32>,
    pub comp: Option<i32>,
    pub cache_model: Option<String>,
    pub cache_size: Option<i32>,
}

/// CREATE TABLE/STABLE statement
#[derive(Debug, Clone)]
pub struct CreateTableStatement {
    pub if_not_exists: bool,
    pub table: TableRef,
    pub columns: Vec<ColumnDef>,
    pub tags: Option<Vec<ColumnDef>>,
    pub using: Option<UsingClause>,
}

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub comment: Option<String>,
}

/// Data types
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Timestamp,
    Bool,
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    UTinyInt,
    USmallInt,
    UInt,
    UBigInt,
    Float,
    Double,
    Binary(u32),
    NChar(u32),
    VarBinary(u32),
    Json,
    Geometry(u32),
}

impl DataType {
    /// Parse data type from string
    pub fn from_str(s: &str) -> Option<Self> {
        let s_upper = s.to_uppercase();
        
        // Handle types with length
        if s_upper.starts_with("BINARY(") || s_upper.starts_with("NCHAR(") 
           || s_upper.starts_with("VARBINARY(") || s_upper.starts_with("GEOMETRY(") {
            let (type_name, len_str) = s_upper.split_once('(').unwrap();
            let len: u32 = len_str.trim_end_matches(')').parse().ok()?;
            
            return match type_name {
                "BINARY" => Some(DataType::Binary(len)),
                "NCHAR" => Some(DataType::NChar(len)),
                "VARBINARY" => Some(DataType::VarBinary(len)),
                "GEOMETRY" => Some(DataType::Geometry(len)),
                _ => None,
            };
        }
        
        match s_upper.as_str() {
            "TIMESTAMP" => Some(DataType::Timestamp),
            "BOOL" | "BOOLEAN" => Some(DataType::Bool),
            "TINYINT" | "INT8" => Some(DataType::TinyInt),
            "SMALLINT" | "INT16" => Some(DataType::SmallInt),
            "INT" | "INT32" => Some(DataType::Int),
            "BIGINT" | "INT64" => Some(DataType::BigInt),
            "TINYINT UNSIGNED" | "UINT8" => Some(DataType::UTinyInt),
            "SMALLINT UNSIGNED" | "UINT16" => Some(DataType::USmallInt),
            "INT UNSIGNED" | "UINT32" => Some(DataType::UInt),
            "BIGINT UNSIGNED" | "UINT64" => Some(DataType::UBigInt),
            "FLOAT" => Some(DataType::Float),
            "DOUBLE" => Some(DataType::Double),
            "JSON" => Some(DataType::Json),
            _ => None,
        }
    }
    
    /// Get size in bytes
    pub fn size(&self) -> usize {
        match self {
            DataType::Bool => 1,
            DataType::TinyInt | DataType::UTinyInt => 1,
            DataType::SmallInt | DataType::USmallInt => 2,
            DataType::Int | DataType::UInt | DataType::Float => 4,
            DataType::BigInt | DataType::UBigInt | DataType::Double | DataType::Timestamp => 8,
            DataType::Binary(len) | DataType::NChar(len) | DataType::VarBinary(len) => *len as usize,
            DataType::Geometry(len) => *len as usize,
            DataType::Json => 4096, // Default JSON size
        }
    }
}

/// CREATE STREAM statement
#[derive(Debug, Clone)]
pub struct CreateStreamStatement {
    pub if_not_exists: bool,
    pub name: String,
    pub target_table: TableRef,
    pub options: StreamOptions,
    pub query: SelectStatement,
}

/// Stream options
#[derive(Debug, Clone, Default)]
pub struct StreamOptions {
    pub trigger: Option<String>,
    pub watermark: Option<String>,
    pub ignore_expired: Option<bool>,
    pub delete_mark: Option<String>,
    pub fill_history: Option<bool>,
    pub ignore_update: Option<bool>,
}

/// Parsed query with metadata
#[derive(Debug, Clone)]
pub struct ParsedQuery {
    pub statement: Statement,
    pub tables_referenced: Vec<String>,
    pub has_window: bool,
    pub has_partition: bool,
    pub estimated_complexity: u32,
}

/// SQL Statement types
#[derive(Debug, Clone)]
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    CreateDatabase(CreateDatabaseStatement),
    CreateTable(CreateTableStatement),
    CreateStream(CreateStreamStatement),
    Use(String),
    ShowDatabases,
    ShowTables(Option<String>),
    ShowStables(Option<String>),
    Describe(TableRef),
    Drop(DropStatement),
    Alter(AlterStatement),
    Explain(Box<Statement>),
}

/// DROP statement
#[derive(Debug, Clone)]
pub struct DropStatement {
    pub object_type: ObjectType,
    pub if_exists: bool,
    pub name: String,
}

/// Object types for DROP
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectType {
    Database,
    Table,
    Stable,
    Stream,
    Topic,
    Index,
}

/// ALTER statement
#[derive(Debug, Clone)]
pub struct AlterStatement {
    pub object_type: ObjectType,
    pub name: TableRef,
    pub action: AlterAction,
}

/// ALTER actions
#[derive(Debug, Clone)]
pub enum AlterAction {
    AddColumn(ColumnDef),
    DropColumn(String),
    ModifyColumn(ColumnDef),
    AddTag(ColumnDef),
    DropTag(String),
    ModifyTag(String, Expr),
    Rename(String),
}
