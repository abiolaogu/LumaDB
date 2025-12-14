
use crate::storage::columnar::ColumnChunk;
use crate::indexing::bitmap::BitmapIndex;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub type SegmentId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    pub id: SegmentId,
    pub time_range: (i64, i64),
    pub columns: HashMap<String, ColumnChunk>,
    pub metadata: SegmentMetadata,
    
    // Indices
    pub time_index: TimeIndex,
    pub label_index: InvertedIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentMetadata {
    pub created_at: i64,
    pub row_count: u64,
    pub size_bytes: u64,
    pub partition_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeIndex {
    // Sparse index: Timestamp -> Row Offset
    pub entries: Vec<TimeIndexEntry>,
    pub granularity_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeIndexEntry {
    pub timestamp: i64,
    pub row_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvertedIndex {
    // Label Name -> Label Value -> Bitmap of Row IDs
    pub index: HashMap<String, HashMap<String, BitmapIndex>>,
}

impl Segment {
    pub fn new(id: SegmentId, time_range: (i64, i64)) -> Self {
        Self {
            id,
            time_range,
            columns: HashMap::new(),
            metadata: SegmentMetadata {
                created_at: 0, // Should be current time
                row_count: 0,
                size_bytes: 0,
                partition_key: None,
            },
            time_index: TimeIndex { entries: vec![], granularity_sec: 60 },
            label_index: InvertedIndex { index: HashMap::new() },
        }
    }
}
