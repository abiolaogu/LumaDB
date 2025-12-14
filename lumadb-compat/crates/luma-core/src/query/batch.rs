
use crate::types::Value;
use std::sync::Arc;
use arrow::record_batch::RecordBatch;
use arrow::array::{Array, Float64Array, Int64Array, StringArray, BinaryArray, BooleanArray};
use arrow::datatypes::{Schema, Field, DataType};

/// A columnar batch of data, backed by Arrow RecordBatch.
/// This enables full interoperability with the Arrow ecosystem.
#[derive(Debug, Clone)]
pub struct LumaBatch {
    pub inner: RecordBatch,
}

impl LumaBatch {
    pub fn new(batch: RecordBatch) -> Self {
        Self { inner: batch }
    }
    
    pub fn row_count(&self) -> usize {
        self.inner.num_rows()
    }
    
    pub fn schema(&self) -> Vec<String> {
        self.inner.schema().fields().iter().map(|f| f.name().clone()).collect()
    }

    /// Iterator over rows (converted to legacy Luma Value format)
    /// Note: This is expensive and should only be used for final output or debugging.
    pub fn rows(&self) -> impl Iterator<Item = Vec<Value>> + '_ {
        (0..self.inner.num_rows()).map(move |row_idx| {
            self.inner.columns().iter().map(|col| {
                if col.is_null(row_idx) {
                    return Value::Null;
                }
                
                match col.data_type() {
                    DataType::Float64 => {
                       let arr = col.as_any().downcast_ref::<Float64Array>().unwrap();
                       Value::Float64(arr.value(row_idx))
                    },
                    DataType::Int64 => {
                       let arr = col.as_any().downcast_ref::<Int64Array>().unwrap();
                       Value::Int64(arr.value(row_idx))
                    },
                    DataType::Utf8 => {
                       let arr = col.as_any().downcast_ref::<StringArray>().unwrap();
                       Value::Text(arr.value(row_idx).to_string())
                    },
                    DataType::Boolean => {
                       let arr = col.as_any().downcast_ref::<BooleanArray>().unwrap();
                       Value::Bool(arr.value(row_idx)) 
                    },
                    _ => Value::Null, // TODO: Support all types
                }
            }).collect()
        })
    }
}
