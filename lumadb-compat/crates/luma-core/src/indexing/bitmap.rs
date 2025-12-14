
use std::collections::{HashMap, HashSet};
use crate::Result;
use serde::{Serialize, Deserialize};

/// A simplified Bitmap Index using HashSet (mocking RoaringBitmap behavior).
/// In production, this should be replaced with `roaring::RoaringBitmap`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SimpleBitmap {
    bits: HashSet<u32>,
}

impl SimpleBitmap {
    pub fn new() -> Self {
        Self { bits: HashSet::new() }
    }

    pub fn insert(&mut self, value: u32) {
        self.bits.insert(value);
    }
    
    pub fn contains(&self, value: u32) -> bool {
        self.bits.contains(&value)
    }

    pub fn union_with(&mut self, other: &SimpleBitmap) {
        for &bit in &other.bits {
            self.bits.insert(bit);
        }
    }

    pub fn serialized_size(&self) -> usize {
        self.bits.len() * 4
    }
}

/// A Bitmap Index for a column.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BitmapIndex {
    index: HashMap<u64, SimpleBitmap>,
    row_count: usize,
}

impl BitmapIndex {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            row_count: 0,
        }
    }

    /// Add a value for a specific row ID
    pub fn insert(&mut self, value_hash: u64, row_id: u32) {
        self.index
            .entry(value_hash)
            .or_insert_with(SimpleBitmap::new)
            .insert(row_id);
        
        if row_id as usize >= self.row_count {
            self.row_count = row_id as usize + 1;
        }
    }

    /// Get row IDs matching the value hash
    pub fn lookup(&self, value_hash: u64) -> Option<SimpleBitmap> {
        self.index.get(&value_hash).cloned()
    }

    /// Get row IDs matching ANY of the value hashes (OR)
    pub fn lookup_any(&self, value_hashes: &[u64]) -> SimpleBitmap {
        let mut result = SimpleBitmap::new();
        for hash in value_hashes {
            if let Some(bitmap) = self.index.get(hash) {
                result.union_with(bitmap);
            }
        }
        result
    }

    pub fn size_in_bytes(&self) -> usize {
        self.index.values().map(|b| b.serialized_size()).sum()
    }
}
