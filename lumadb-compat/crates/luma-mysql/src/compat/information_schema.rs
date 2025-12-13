// Information Schema Virtual Tables

pub const TABLES_COLS: &[(&str, u8)] = &[
    ("TABLE_CATALOG", 0xfd), // VarString
    ("TABLE_SCHEMA", 0xfd),
    ("TABLE_NAME", 0xfd),
    ("TABLE_TYPE", 0xfd),
    ("ENGINE", 0xfd),
    ("VERSION", 0x08), // LongLong
    ("ROW_FORMAT", 0xfd),
    ("TABLE_ROWS", 0x08),
    ("AVG_ROW_LENGTH", 0x08),
    ("DATA_LENGTH", 0x08),
    ("MAX_DATA_LENGTH", 0x08),
    ("INDEX_LENGTH", 0x08),
    ("DATA_FREE", 0x08),
    ("AUTO_INCREMENT", 0x08),
    ("CREATE_TIME", 0x0c), // DateTime
    ("UPDATE_TIME", 0x0c),
    ("CHECK_TIME", 0x0c),
    ("TABLE_COLLATION", 0xfd),
    ("CHECKSUM", 0x08),
    ("CREATE_OPTIONS", 0xfd),
    ("TABLE_COMMENT", 0xfd),
];

// Placeholder for logic to generate these rows using luma-core catalog
