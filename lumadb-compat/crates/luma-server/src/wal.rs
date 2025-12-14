//! Write-Ahead Log (WAL) for Durability
//! Provides crash recovery and data persistence

use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

/// WAL Entry Types
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WalEntryType {
    /// Key-value set operation
    Set { key: String, value: Vec<u8> },
    /// Key deletion
    Delete { key: String },
    /// Batch operation
    Batch { entries: Vec<WalEntry> },
    /// Checkpoint marker
    Checkpoint { sequence: u64 },
}

/// Single WAL entry with metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalEntry {
    /// Sequence number
    pub sequence: u64,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: u64,
    /// Entry type and data
    pub entry_type: WalEntryType,
    /// CRC32 checksum
    pub checksum: u32,
}

impl WalEntry {
    pub fn new(sequence: u64, entry_type: WalEntryType) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        let mut entry = Self {
            sequence,
            timestamp,
            entry_type,
            checksum: 0,
        };
        entry.checksum = entry.compute_checksum();
        entry
    }

    fn compute_checksum(&self) -> u32 {
        let data = format!("{}{}{:?}", self.sequence, self.timestamp, self.entry_type);
        crc32fast::hash(data.as_bytes())
    }

    pub fn verify(&self) -> bool {
        let expected = self.compute_checksum();
        self.checksum == expected
    }
}

/// WAL Segment (individual log file)
pub struct WalSegment {
    path: PathBuf,
    writer: BufWriter<File>,
    min_sequence: u64,
    max_sequence: u64,
    size: u64,
}

impl WalSegment {
    pub fn create(dir: &Path, sequence: u64) -> io::Result<Self> {
        let path = dir.join(format!("wal-{:020}.log", sequence));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;
        
        Ok(Self {
            path,
            writer: BufWriter::new(file),
            min_sequence: sequence,
            max_sequence: sequence,
            size: 0,
        })
    }

    pub fn append(&mut self, entry: &WalEntry) -> io::Result<()> {
        let data = serde_json::to_vec(entry)?;
        let len = data.len() as u32;
        
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&data)?;
        self.writer.write_all(b"\n")?;
        
        self.max_sequence = entry.sequence;
        self.size += 4 + data.len() as u64 + 1;
        
        Ok(())
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.writer.flush()?;
        self.writer.get_ref().sync_all()
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

/// WAL Configuration
#[derive(Clone, Debug)]
pub struct WalConfig {
    /// Directory for WAL files
    pub dir: PathBuf,
    /// Maximum segment size (bytes)
    pub max_segment_size: u64,
    /// Sync mode
    pub sync_mode: WalSyncMode,
    /// Maximum segments to keep
    pub max_segments: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WalSyncMode {
    /// Sync after every write (safest, slowest)
    EveryWrite,
    /// Sync after N writes
    EveryN(usize),
    /// Sync periodically (fastest, least safe)
    Periodic,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from("./data/wal"),
            max_segment_size: 64 * 1024 * 1024, // 64MB
            sync_mode: WalSyncMode::EveryN(100),
            max_segments: 10,
        }
    }
}

/// Write-Ahead Log Manager
pub struct Wal {
    config: WalConfig,
    current_segment: RwLock<Option<WalSegment>>,
    sequence: std::sync::atomic::AtomicU64,
    writes_since_sync: std::sync::atomic::AtomicUsize,
    segments: RwLock<Vec<PathBuf>>,
}

impl Wal {
    pub fn new(config: WalConfig) -> io::Result<Arc<Self>> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(&config.dir)?;
        
        // Find existing segments
        let mut segments: Vec<PathBuf> = std::fs::read_dir(&config.dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "log").unwrap_or(false))
            .collect();
        segments.sort();
        
        // Find last sequence number
        let last_sequence = Self::find_last_sequence(&segments)?;
        
        info!("WAL initialized with {} segments, last sequence: {}", 
            segments.len(), last_sequence);
        
        Ok(Arc::new(Self {
            config,
            current_segment: RwLock::new(None),
            sequence: std::sync::atomic::AtomicU64::new(last_sequence + 1),
            writes_since_sync: std::sync::atomic::AtomicUsize::new(0),
            segments: RwLock::new(segments),
        }))
    }

    fn find_last_sequence(segments: &[PathBuf]) -> io::Result<u64> {
        if segments.is_empty() {
            return Ok(0);
        }
        
        // Read last segment to find max sequence
        let last = segments.last().unwrap();
        let mut max_seq = 0u64;
        
        for entry in Self::read_segment(last)? {
            if entry.sequence > max_seq {
                max_seq = entry.sequence;
            }
        }
        
        Ok(max_seq)
    }

    fn read_segment(path: &Path) -> io::Result<Vec<WalEntry>> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();
        
        loop {
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf) {
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            }
            
            let len = u32::from_le_bytes(len_buf) as usize;
            let mut data = vec![0u8; len];
            reader.read_exact(&mut data)?;
            
            // Skip newline
            let mut newline = [0u8; 1];
            let _ = reader.read_exact(&mut newline);
            
            match serde_json::from_slice::<WalEntry>(&data) {
                Ok(entry) => {
                    if entry.verify() {
                        entries.push(entry);
                    } else {
                        warn!("WAL entry checksum mismatch, skipping");
                    }
                }
                Err(e) => {
                    warn!("Failed to parse WAL entry: {}", e);
                }
            }
        }
        
        Ok(entries)
    }

    /// Append an entry to the WAL
    pub async fn append(&self, entry_type: WalEntryType) -> io::Result<u64> {
        let sequence = self.sequence.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let entry = WalEntry::new(sequence, entry_type);
        
        {
            let mut segment_guard = self.current_segment.write().await;
            
            // Create new segment if needed
            if segment_guard.is_none() || 
               segment_guard.as_ref().map(|s| s.size() >= self.config.max_segment_size).unwrap_or(false) {
                if let Some(old) = segment_guard.take() {
                    drop(old);
                }
                let new_segment = WalSegment::create(&self.config.dir, sequence)?;
                *segment_guard = Some(new_segment);
                
                let mut segments = self.segments.write().await;
                segments.push(self.config.dir.join(format!("wal-{:020}.log", sequence)));
                
                // Cleanup old segments
                while segments.len() > self.config.max_segments {
                    if let Some(old_path) = segments.first() {
                        let _ = std::fs::remove_file(old_path);
                        segments.remove(0);
                    }
                }
            }
            
            if let Some(ref mut segment) = *segment_guard {
                segment.append(&entry)?;
                
                // Handle sync mode
                let writes = self.writes_since_sync.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                match &self.config.sync_mode {
                    WalSyncMode::EveryWrite => segment.sync()?,
                    WalSyncMode::EveryN(n) if writes >= *n => {
                        segment.sync()?;
                        self.writes_since_sync.store(0, std::sync::atomic::Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        }
        
        debug!("WAL append sequence {}", sequence);
        Ok(sequence)
    }

    /// Sync the current segment to disk
    pub async fn sync(&self) -> io::Result<()> {
        let mut segment_guard = self.current_segment.write().await;
        if let Some(ref mut segment) = *segment_guard {
            segment.sync()?;
        }
        Ok(())
    }

    /// Replay WAL entries from a given sequence
    pub async fn replay(&self, from_sequence: u64) -> io::Result<Vec<WalEntry>> {
        let segments = self.segments.read().await;
        let mut entries = Vec::new();
        
        for path in segments.iter() {
            for entry in Self::read_segment(path)? {
                if entry.sequence >= from_sequence {
                    entries.push(entry);
                }
            }
        }
        
        entries.sort_by_key(|e| e.sequence);
        info!("Replayed {} WAL entries from sequence {}", entries.len(), from_sequence);
        
        Ok(entries)
    }

    /// Create a checkpoint
    pub async fn checkpoint(&self) -> io::Result<u64> {
        let sequence = self.append(WalEntryType::Checkpoint { 
            sequence: self.sequence.load(std::sync::atomic::Ordering::SeqCst) 
        }).await?;
        self.sync().await?;
        info!("WAL checkpoint at sequence {}", sequence);
        Ok(sequence)
    }

    /// Get current sequence number
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_wal_append_and_replay() {
        let dir = tempdir().unwrap();
        let config = WalConfig {
            dir: dir.path().to_path_buf(),
            max_segment_size: 1024,
            sync_mode: WalSyncMode::EveryWrite,
            max_segments: 5,
        };
        
        let wal = Wal::new(config).unwrap();
        
        // Append some entries
        wal.append(WalEntryType::Set { 
            key: "key1".to_string(), 
            value: b"value1".to_vec() 
        }).await.unwrap();
        
        wal.append(WalEntryType::Set { 
            key: "key2".to_string(), 
            value: b"value2".to_vec() 
        }).await.unwrap();
        
        wal.append(WalEntryType::Delete { 
            key: "key1".to_string() 
        }).await.unwrap();
        
        // Replay
        let entries = wal.replay(0).await.unwrap();
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_wal_entry_checksum() {
        let entry = WalEntry::new(1, WalEntryType::Set {
            key: "test".to_string(),
            value: vec![1, 2, 3],
        });
        
        assert!(entry.verify());
    }
}
