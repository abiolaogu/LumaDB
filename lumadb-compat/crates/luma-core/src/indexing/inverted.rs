
use dashmap::DashMap;
use roaring::RoaringBitmap;
use std::sync::{Arc, RwLock};
use std::collections::{BTreeMap, BTreeSet};

/// LumaText: Lightweight Inverted Index for High-Performance Search.
/// 
/// Comparison vs Lucene (Elasticsearch):
/// - Data Structure: RoaringBitmap (Fast Set Operations) vs RoaringDocIdSet (Lucene)
/// - Dictionary: BTree (Simpler/Faster Updates) vs FST (Compact/Immutable in Lucene segments)
/// - Concurrency: Lock-Free Reads (DashMap) vs Segment-based merging
/// 
/// This implementation prioritizes write throughput and low latency reads for "Log Scale" data.
#[derive(Debug, Clone)]
pub struct InvertedIndex {
    /// Token -> Bitmap of Document IDs
    /// We use DashMap for high concurrency.
    /// In a real FST implementation, we would flush this map to immutable FST segments.
    index: Arc<DashMap<String, RoaringBitmap>>,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            index: Arc::new(DashMap::new()),
        }
    }

    /// Add document to index.
    /// Tokenizes text and updates bitmaps.
    pub fn add_document(&self, doc_id: u32, text: &str) {
        let tokens = self.tokenize(text);
        for token in tokens {
            // DashMap entry API handles locking per key
            self.index.entry(token)
                .or_insert_with(RoaringBitmap::new)
                .insert(doc_id);
        }
    }

    /// Search for a term.
    /// Returns a Bitmap of matching Document IDs.
    pub fn search(&self, term: &str) -> Option<RoaringBitmap> {
        self.index.get(term).map(|bitmap| bitmap.clone())
    }

    /// Boolean AND search (intersection)
    pub fn search_and(&self, terms: Vec<&str>) -> RoaringBitmap {
        let mut result: Option<RoaringBitmap> = None;

        for term in terms {
            if let Some(bitmap) = self.search(term) {
                match result {
                    None => result = Some(bitmap),
                    Some(ref mut res) => *res &= bitmap,
                }
            } else {
                return RoaringBitmap::new(); // Short circuit
            }
        }
        
        result.unwrap_or_else(RoaringBitmap::new)
    }

    /// Boolean OR search (union)
    pub fn search_or(&self, terms: Vec<&str>) -> RoaringBitmap {
        let mut result = RoaringBitmap::new();
        for term in terms {
            if let Some(bitmap) = self.search(term) {
                result |= bitmap;
            }
        }
        result
    }

    /// Simple tokenizer (lowercase, split by whitespace and punctuation)
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }
}
