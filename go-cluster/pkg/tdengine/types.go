// Package tdengine provides TDengine-compatible REST API for LumaDB
package tdengine

import "time"

// Response structures matching TDengine format exactly
type Response struct {
	Code         int             `json:"code"`
	Desc         string          `json:"desc,omitempty"`
	ColumnMeta   [][]interface{} `json:"column_meta,omitempty"`
	Data         [][]interface{} `json:"data,omitempty"`
	Rows         int             `json:"rows,omitempty"`
	AffectedRows int             `json:"affected_rows,omitempty"`
}

// TimingResponse includes execution timing (for /rest/sqlt)
type TimingResponse struct {
	Response
	Timing int64 `json:"timing"` // Execution time in microseconds
}

// OpenTSDBPoint represents an OpenTSDB data point
type OpenTSDBPoint struct {
	Metric    string            `json:"metric"`
	Timestamp int64             `json:"timestamp"`
	Value     float64           `json:"value"`
	Tags      map[string]string `json:"tags"`
}

// ExecuteOptions contains SQL execution options
type ExecuteOptions struct {
	ReqID       string
	Timezone    string
	RowWithMeta bool
}

// ColumnMeta represents column metadata
type ColumnMeta struct {
	Name   string `json:"name"`
	Type   int    `json:"type"`
	Length int    `json:"length"`
}

// TDengine data type constants
const (
	TSDB_DATA_TYPE_NULL      = 0
	TSDB_DATA_TYPE_BOOL      = 1
	TSDB_DATA_TYPE_TINYINT   = 2
	TSDB_DATA_TYPE_SMALLINT  = 3
	TSDB_DATA_TYPE_INT       = 4
	TSDB_DATA_TYPE_BIGINT    = 5
	TSDB_DATA_TYPE_FLOAT     = 6
	TSDB_DATA_TYPE_DOUBLE    = 7
	TSDB_DATA_TYPE_BINARY    = 8
	TSDB_DATA_TYPE_TIMESTAMP = 9
	TSDB_DATA_TYPE_NCHAR     = 10
	TSDB_DATA_TYPE_UTINYINT  = 11
	TSDB_DATA_TYPE_USMALLINT = 12
	TSDB_DATA_TYPE_UINT      = 13
	TSDB_DATA_TYPE_UBIGINT   = 14
	TSDB_DATA_TYPE_JSON      = 15
	TSDB_DATA_TYPE_VARBINARY = 16
	TSDB_DATA_TYPE_DECIMAL   = 17
	TSDB_DATA_TYPE_BLOB      = 18
	TSDB_DATA_TYPE_MEDIUMBLOB= 19
	TSDB_DATA_TYPE_GEOMETRY  = 20
)

// Error codes matching TDengine
const (
	TSDB_CODE_SUCCESS               = 0
	TSDB_CODE_FAILED                = 0x80000001
	TSDB_CODE_ACTION_IN_PROGRESS    = 0x80000002
	TSDB_CODE_TSC_INVALID_SQL       = 0x80000200
	TSDB_CODE_MND_DB_NOT_EXIST      = 0x80000388
	TSDB_CODE_MND_INVALID_DB        = 0x80000383
	TSDB_CODE_MND_TABLE_NOT_EXIST   = 0x80000390
	TSDB_CODE_MND_INVALID_TABLE     = 0x80000391
	TSDB_CODE_TSC_AUTH_FAILURE      = 0x80000357
	TSDB_CODE_TSC_INVALID_OPERATION = 0x80000356
)

// Database represents a TDengine database
type Database struct {
	Name        string
	Precision   string // "ms", "us", "ns"
	Buffer      int
	Pages       int
	PageSize    int
	MinRows     int
	MaxRows     int
	WAL         int
	Comp        int
	Replica     int
	Keep        string // e.g., "3650d,3650d,3650d"
	CacheModel  string
	CacheSize   int
	STables     map[string]*SuperTable
	Tables      map[string]*Table
	CreatedAt   time.Time
}

// SuperTable represents a TDengine super table (template)
type SuperTable struct {
	Name      string
	Schema    []Column
	Tags      []Column
	SubTables map[string]*Table
	CreatedAt time.Time
}

// Table represents a TDengine table (or child table)
type Table struct {
	Name       string
	Schema     []Column
	Tags       map[string]interface{} // Tag values for subtables
	SuperTable string                 // Parent supertable (if any)
	CreatedAt  time.Time
}

// Column represents a table column
type Column struct {
	Name     string
	Type     int
	Length   int
	IsTag    bool
	Nullable bool
}

// DataRow represents a row of data
type DataRow struct {
	Timestamp int64
	Values    []interface{}
	Tags      map[string]interface{}
}

// WindowType enumeration
type WindowType int

const (
	WindowInterval WindowType = iota
	WindowSession
	WindowState
	WindowEvent
	WindowCount
)

// FillType enumeration
type FillType int

const (
	FillNone FillType = iota
	FillNull
	FillValue
	FillPrev
	FillNext
	FillLinear
)

// StreamDefinition represents a TDengine stream
type StreamDefinition struct {
	Name        string
	SourceTable string
	TargetTable string
	SQL         string
	Trigger     string // "at_once", "window_close", "max_delay"
	Watermark   string
	IgnoreExpired bool
	DeleteMark  string
	FillHistory bool
	IgnoreUpdate bool
	CreatedAt   time.Time
}

// TopicDefinition represents a TDengine topic (for TMQ)
type TopicDefinition struct {
	Name      string
	Database  string
	SQL       string
	WithMeta  bool
	CreatedAt time.Time
}

// UserDefinition represents a TDengine user
type UserDefinition struct {
	Name      string
	Password  string // Hashed
	Privilege string // "super", "read", "write"
	Database  string
	CreatedAt time.Time
}

// InfluxDBLineProtocol represents parsed InfluxDB line protocol data
type InfluxDBLineProtocol struct {
	Measurement string
	Tags        map[string]string
	Fields      map[string]interface{}
	Timestamp   int64
}

// QueryPlan represents a query execution plan
type QueryPlan struct {
	Nodes     []PlanNode
	Estimated int64
}

// PlanNode represents a node in the query plan
type PlanNode struct {
	ID        int
	Name      string
	NodeType  string
	Cost      float64
	Rows      int64
	Width     int
	Children  []int
}
