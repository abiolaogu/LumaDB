//! Unit tests for Write-Ahead Log

use luma_protocol_core::storage::wal::{WalManager, WalEntry};
use luma_protocol_core::storage::segment::Segment;
use std::path::PathBuf;
use tempfile::tempdir;

#[tokio::test]
async fn test_wal_write_and_recover() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wal_path = dir.path().join("test_wal.log");
    
    // Create WAL and write segments
    {
        let wal = WalManager::new(wal_path.clone()).await.expect("Failed to create WAL");
        
        let seg1 = Segment::new("seg1".to_string(), (1000, 2000));
        let seg2 = Segment::new("seg2".to_string(), (2000, 3000));
        
        wal.append_segment(&seg1).await.expect("Failed to append seg1");
        wal.append_segment(&seg2).await.expect("Failed to append seg2");
    }
    
    // Recover from WAL
    let recovered = WalManager::recover(wal_path).await.expect("Recovery failed");
    
    assert_eq!(recovered.len(), 2, "Should recover 2 segments");
    assert_eq!(recovered[0].id, "seg1");
    assert_eq!(recovered[1].id, "seg2");
}

#[tokio::test]
async fn test_wal_recover_empty() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wal_path = dir.path().join("nonexistent_wal.log");
    
    // Recover from non-existent WAL (should return empty)
    let recovered = WalManager::recover(wal_path).await.expect("Recovery failed");
    
    assert!(recovered.is_empty(), "Should recover 0 segments from non-existent file");
}

#[tokio::test]
async fn test_wal_multiple_writes() {
    let dir = tempdir().expect("Failed to create temp dir");
    let wal_path = dir.path().join("multi_wal.log");
    
    let wal = WalManager::new(wal_path.clone()).await.expect("Failed to create WAL");
    
    for i in 0..100 {
        let seg = Segment::new(format!("seg_{}", i), (i * 1000, (i + 1) * 1000));
        wal.append_segment(&seg).await.expect("Failed to append segment");
    }
    
    drop(wal);
    
    let recovered = WalManager::recover(wal_path).await.expect("Recovery failed");
    assert_eq!(recovered.len(), 100, "Should recover 100 segments");
}
