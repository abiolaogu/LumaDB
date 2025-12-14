
use crate::storage::segment::Segment;
use crate::Value;
use std::collections::HashMap;

pub fn segment_to_rows(segment: &Segment) -> Vec<HashMap<String, Value>> {
    // Simplified conversion for stream triggering
    // In production this would be zero-copy using arrow arrays directly
    let row_count = segment.metadata.row_count as usize;
    let mut rows = Vec::with_capacity(row_count);
    
    for _ in 0..row_count {
        rows.push(HashMap::new());
    }
    
    // Iterate columns and fill rows (Transpose)
    // This is valid since Segment is columnar
    // Mock implementation for now as actual column access needs decompression logic
    // We assume raw data access is abstracted or handled elsewhere in a real impl
    rows
}
