
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter, BufReader};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::storage::segment::Segment;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum WalEntry {
    InsertSegment(Segment),
    // DropSegment(String),
}

pub struct WalManager {
    writer: Arc<Mutex<BufWriter<File>>>,
}

impl WalManager {
    pub async fn new(path: PathBuf) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
            
        Ok(Self {
            writer: Arc::new(Mutex::new(BufWriter::new(file))),
        })
    }
    
    pub async fn append_segment(&self, segment: &Segment) -> Result<(), std::io::Error> {
        let entry = WalEntry::InsertSegment(segment.clone());
        let data = serde_json::to_vec(&entry)?; // Use JSON for simplicity, Bincode/Protobuf better for prod
        
        let mut writer = self.writer.lock().await;
        // Format: [Length: u32][Data: bytes]
        writer.write_u32(data.len() as u32).await?;
        writer.write_all(&data).await?;
        writer.flush().await?;
        Ok(())
    }
    
    /// Recover segments from WAL file
    /// Reads length-prefixed entries and deserializes them
    pub async fn recover(path: PathBuf) -> Result<Vec<Segment>, std::io::Error> {
        let file = match File::open(&path).await {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(vec![]); // No WAL file = nothing to recover
            }
            Err(e) => return Err(e),
        };
        
        let mut reader = BufReader::new(file);
        let mut segments = Vec::new();
        
        loop {
            // Read length prefix
            let len = match reader.read_u32().await {
                Ok(l) => l as usize,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break, // End of file
                Err(e) => {
                    tracing::warn!("WAL recovery: error reading length: {}", e);
                    break; // Corrupt entry, stop recovery
                }
            };
            
            // Read entry data
            let mut data = vec![0u8; len];
            if let Err(e) = reader.read_exact(&mut data).await {
                tracing::warn!("WAL recovery: incomplete entry, stopping: {}", e);
                break; // Partial entry, stop recovery
            }
            
            // Deserialize entry
            match serde_json::from_slice::<WalEntry>(&data) {
                Ok(WalEntry::InsertSegment(segment)) => {
                    segments.push(segment);
                }
                Err(e) => {
                    tracing::warn!("WAL recovery: failed to deserialize entry: {}", e);
                    // Continue to next entry (best effort recovery)
                }
            }
        }
        
        tracing::info!("WAL recovery: recovered {} segments", segments.len());
        Ok(segments)
    }
}
