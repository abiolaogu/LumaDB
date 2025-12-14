//! Unit tests for core storage and indexing modules

use luma_protocol_core::storage::metric_store::MetricsStorage;
use luma_protocol_core::indexing::inverted::InvertedIndex;
use std::collections::HashMap;

#[tokio::test]
async fn test_metrics_storage_insert_and_query() {
    let storage = MetricsStorage::new();
    
    let mut labels = HashMap::new();
    labels.insert("env".to_string(), "prod".to_string());
    
    // Insert sample
    let result = storage.insert_sample("http_requests_total", labels, 1000, 42.0).await;
    assert!(result.is_ok(), "Insert should succeed");
}

#[tokio::test]
async fn test_metrics_storage_multiple_inserts() {
    let storage = MetricsStorage::new();
    
    for i in 0..10 {
        let mut labels = HashMap::new();
        labels.insert("env".to_string(), "test".to_string());
        let result = storage.insert_sample("counter", labels, 1000 + i, i as f64).await;
        assert!(result.is_ok());
    }
}

#[test]
fn test_inverted_index_add_and_search() {
    let index = InvertedIndex::new();
    
    index.add_document(1, "hello world");
    index.add_document(2, "hello rust");
    index.add_document(3, "rust world programming");
    
    // Single term search
    let results = index.search("hello");
    assert!(results.is_some());
    let bitmap = results.unwrap();
    assert!(bitmap.contains(1));
    assert!(bitmap.contains(2));
    assert!(!bitmap.contains(3));
}

#[test]
fn test_inverted_index_and_search() {
    let index = InvertedIndex::new();
    
    index.add_document(1, "rust programming language");
    index.add_document(2, "rust systems programming");
    index.add_document(3, "python programming");
    
    // AND search
    let results = index.search_and(vec!["rust", "programming"]);
    assert!(results.contains(1));
    assert!(results.contains(2));
    assert!(!results.contains(3));
}

#[test]
fn test_inverted_index_or_search() {
    let index = InvertedIndex::new();
    
    index.add_document(1, "hello");
    index.add_document(2, "world");
    index.add_document(3, "foo");
    
    // OR search
    let results = index.search_or(vec!["hello", "world"]);
    assert!(results.contains(1));
    assert!(results.contains(2));
    assert!(!results.contains(3));
}

#[test]
fn test_inverted_index_empty_search() {
    let index = InvertedIndex::new();
    
    index.add_document(1, "hello world");
    
    let results = index.search("nonexistent");
    assert!(results.is_none());
}
