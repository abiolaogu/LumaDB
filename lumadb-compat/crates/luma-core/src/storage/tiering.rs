
use crate::storage::segment::{Segment, SegmentId};
use std::path::PathBuf;
use tokio::fs;
use std::sync::Arc;
use dashmap::DashMap;
use crate::storage::object_store::LocalObjectStore;

// High-Performance Hot Tier using DashMap with Arc<Segment> to avoid cloning
pub struct HotTier {
    segments: Arc<DashMap<SegmentId, Arc<Segment>>>,
}

impl HotTier {
    pub fn new() -> Self {
        Self {
            segments: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, segment: Segment) {
        self.segments.insert(segment.id.clone(), Arc::new(segment));
    }
    
    pub fn insert_arc(&self, segment: Arc<Segment>) {
        self.segments.insert(segment.id.clone(), segment);
    }
    
    pub fn get(&self, id: &str) -> Option<Arc<Segment>> {
        self.segments.get(id).map(|ref_seg| ref_seg.value().clone())
    }
}

pub struct WarmTier {
    base_path: PathBuf,
}

// ColdTier backed by Rust-MinIO (Internal Object Store)
pub struct ColdTier {
    store: Arc<LocalObjectStore>,
}

impl WarmTier {
    pub fn new(base_path: PathBuf) -> Self {
        // Ensure directory exists
        let _ = std::fs::create_dir_all(&base_path);
        Self { base_path }
    }
    
    // Simulate writing to disk (JSON for now, Parquet planned)
    pub async fn store(&self, segment: &Segment) -> Result<(), String> {
        let path = self.base_path.join(format!("{}.seg", segment.id));
        let data = serde_json::to_vec(segment).map_err(|e| e.to_string())?;
        fs::write(path, data).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl ColdTier {
    pub async fn new(root_path: PathBuf) -> Self {
        let store = LocalObjectStore::new(root_path).await.expect("Failed to init Rust-MinIO");
        Self { 
            store: Arc::new(store),
        }
    }
    
    pub async fn store(&self, segment: &Segment) -> Result<(), String> {
        let key = format!("{}.seg", segment.id);
        let data = serde_json::to_vec(segment).map_err(|e| e.to_string())?;
        
        // Use Rust-MinIO to store object
        self.store.put_object(&key, data.into()).await.map_err(|e| e.to_string())?;
        
        println!("(Rust-MinIO) Uploaded segment {} to internal object store", segment.id);
        Ok(())
    }
}

use crate::storage::wal::WalManager;

use crate::indexing::InvertedIndex;

use crate::stream::MaterializedView;
use std::sync::RwLock;
use crate::storage::utils::segment_to_rows;

use crate::storage::metric_store::MetricsStorage;
use crate::storage::trace_store::TraceStorage;
use crate::storage::log_store::LogStorage;

pub struct MultiTierStorage {
    hot: HotTier,
    warm: WarmTier,
    cold: ColdTier,
    wal: Arc<WalManager>,
    pub text_index: Arc<InvertedIndex>,
    pub views: Arc<RwLock<Vec<Arc<MaterializedView>>>>,
    // Phase 16: Observability Storage
    pub metrics: Arc<MetricsStorage>,
    pub traces: Arc<TraceStorage>,
    pub logs: Arc<LogStorage>,
    pub windowed_views: Arc<RwLock<Vec<Arc<crate::stream::window_view::WindowedView>>>>,
}

impl MultiTierStorage {
    pub async fn new(data_dir: PathBuf) -> Self {
        let wal_path = data_dir.join("wal.log");
        let wal = WalManager::new(wal_path).await.expect("Failed to initialize WAL");
        
        let object_store_path = data_dir.join("blob_storage");
        
        Self {
            hot: HotTier::new(),
            warm: WarmTier::new(data_dir.join("warm")),
            cold: ColdTier::new(object_store_path).await,
            wal: Arc::new(wal),
            text_index: Arc::new(InvertedIndex::new()),
            views: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(MetricsStorage::new()),
            traces: Arc::new(TraceStorage::new()),
            logs: Arc::new(LogStorage::new()),
            windowed_views: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    pub async fn ingest(&self, segment: Segment) -> Result<(), String> {
        // 1. Write to WAL (Durability)
        self.wal.append_segment(&segment).await.map_err(|e| e.to_string())?;
        
        // 2. Write to Hot Tier (Visibility)
        self.hot.insert(segment.clone()); // Clone for view processing

        // 3. Trigger Materialized Views (Stream Processing)
        // Convert segment columns to Rows for view processing
        // Optimization: In real system, pass ColumnChunk directly. Here we convert.
        let rows = segment_to_rows(&segment);
        let views = self.views.read()
            .map_err(|e| format!("Failed to acquire views lock: {}", e))?;
        for view in views.iter() {
           view.on_insert(&rows); 
        }
        
        Ok(())
    }
    
    // Logic to move Hot -> Warm -> Cold
    pub async fn flush_to_warm(&self, segment_id: &str) -> Result<(), String> {
        let segment = self.hot.get(segment_id);
        
        if let Some(seg) = segment {
            self.warm.store(&seg).await?;
        }
        Ok(())
    }
    
    pub async fn flush_to_cold(&self, segment_id: &str) -> Result<(), String> {
         let segment = self.hot.get(segment_id);
        
        if let Some(seg) = segment {
            self.cold.store(&seg).await?;
        }
        Ok(())
    }
}
