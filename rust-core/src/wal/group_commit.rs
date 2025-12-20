//! Group Commit WAL
//!
//! High-throughput Write-Ahead Log with group commit for durability batching.

use std::sync::{Arc, Mutex, Condvar};
use std::io::{self, Write};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;

/// Log Sequence Number
pub type Lsn = u64;

/// WAL Entry
#[derive(Clone)]
pub struct WalEntry {
    pub lsn: Lsn,
    pub data: Vec<u8>,
}

/// Config for Group Commit
pub struct GroupCommitConfig {
    pub flush_interval_ms: u64,
    pub max_batch_size: usize,
}

impl Default for GroupCommitConfig {
    fn default() -> Self {
        Self {
            flush_interval_ms: 1,
            max_batch_size: 1000,
        }
    }
}

/// Pending commit handle
pub struct PendingCommit {
    lsn: Lsn,
    committed: Arc<(Mutex<bool>, Condvar)>,
}

impl PendingCommit {
    pub fn wait(&self) -> Lsn {
        let (lock, cvar) = &*self.committed;
        let mut committed = lock.lock().unwrap();
        while !*committed {
            committed = cvar.wait(committed).unwrap();
        }
        self.lsn
    }
}

/// Group Commit WAL
pub struct GroupCommitWal {
    file: Arc<Mutex<File>>,
    next_lsn: Arc<Mutex<Lsn>>,
    pending_batch: Arc<Mutex<Vec<(WalEntry, Arc<(Mutex<bool>, Condvar)>)>>>,
    running: Arc<Mutex<bool>>,
}

impl GroupCommitWal {
    pub fn new<P: AsRef<Path>>(path: P, config: GroupCommitConfig) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        
        let wal = Self {
            file: Arc::new(Mutex::new(file)),
            next_lsn: Arc::new(Mutex::new(1)),
            pending_batch: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(Mutex::new(true)),
        };

        // Start flush thread
        let file = Arc::clone(&wal.file);
        let pending = Arc::clone(&wal.pending_batch);
        let running = Arc::clone(&wal.running);
        let interval = Duration::from_millis(config.flush_interval_ms);
        let max_batch = config.max_batch_size;

        thread::spawn(move || {
            while *running.lock().unwrap() {
                thread::sleep(interval);
                
                let batch: Vec<_> = {
                    let mut pending = pending.lock().unwrap();
                    if pending.is_empty() {
                        continue;
                    }
                    pending.drain(..).collect()
                };

                if batch.is_empty() {
                    continue;
                }

                // Write batch
                {
                    let mut file = file.lock().unwrap();
                    for (entry, _) in &batch {
                        // Simple format: [len:4][lsn:8][data]
                        let len = 8 + entry.data.len();
                        file.write_all(&(len as u32).to_le_bytes()).ok();
                        file.write_all(&entry.lsn.to_le_bytes()).ok();
                        file.write_all(&entry.data).ok();
                    }
                    file.sync_all().ok();
                }

                // Notify waiters
                for (_, committed) in batch {
                    let (lock, cvar) = &*committed;
                    let mut done = lock.lock().unwrap();
                    *done = true;
                    cvar.notify_all();
                }
            }
        });

        Ok(wal)
    }

    pub fn append(&self, data: Vec<u8>) -> PendingCommit {
        let lsn = {
            let mut next = self.next_lsn.lock().unwrap();
            let lsn = *next;
            *next += 1;
            lsn
        };

        let entry = WalEntry { lsn, data };
        let committed = Arc::new((Mutex::new(false), Condvar::new()));
        
        {
            let mut batch = self.pending_batch.lock().unwrap();
            batch.push((entry, Arc::clone(&committed)));
        }

        PendingCommit { lsn, committed }
    }

    pub fn append_sync(&self, data: Vec<u8>) -> Lsn {
        self.append(data).wait()
    }
}

impl Drop for GroupCommitWal {
    fn drop(&mut self) {
        *self.running.lock().unwrap() = false;
    }
}
