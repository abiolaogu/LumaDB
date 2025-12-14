
use crate::Result;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DistanceMetric {
    L2,
    Cosine,
    DotProduct,
}

pub trait VectorIndex: Send + Sync {
    /// Add a vector with an associated row ID
    fn add(&mut self, vector: &[f32], row_id: u32) -> Result<()>;
    
    /// Search for k-nearest neighbors
    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(u32, f32)>>;
    
    /// Build/Optimize the index (if applicable)
    fn build(&mut self) -> Result<()>;
    
    /// Get the number of vectors in the index
    fn len(&self) -> usize;
}

/// A Flat Index (Brute Force) implementation.
/// Good baseline for accuracy and small datasets.
pub struct FlatIndex {
    vectors: Vec<Vec<f32>>,
    ids: Vec<u32>,
    metric: DistanceMetric,
}

impl FlatIndex {
    pub fn new(metric: DistanceMetric) -> Self {
        Self {
            vectors: Vec::new(),
            ids: Vec::new(),
            metric,
        }
    }

    fn compute_distance(&self, v1: &[f32], v2: &[f32]) -> f32 {
        match self.metric {
            DistanceMetric::L2 => {
                v1.iter().zip(v2).map(|(a, b)| (a - b).powi(2)).sum()
            }
            DistanceMetric::Cosine => {
                let dot: f32 = v1.iter().zip(v2).map(|(a, b)| a * b).sum();
                let norm1: f32 = v1.iter().map(|a| a.powi(2)).sum::<f32>().sqrt();
                let norm2: f32 = v2.iter().map(|b| b.powi(2)).sum::<f32>().sqrt();
                if norm1 == 0.0 || norm2 == 0.0 {
                    0.0
                } else {
                    1.0 - (dot / (norm1 * norm2))
                }
            }
            DistanceMetric::DotProduct => {
                 let dot: f32 = v1.iter().zip(v2).map(|(a, b)| a * b).sum();
                 -dot // Optimization so smaller is better for sorting (if applicable) or handle uniformly
            }
        }
    }
}

impl VectorIndex for FlatIndex {
    fn add(&mut self, vector: &[f32], row_id: u32) -> Result<()> {
        self.vectors.push(vector.to_vec());
        self.ids.push(row_id);
        Ok(())
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(u32, f32)>> {
        let mut scores: Vec<(u32, f32)> = self.vectors.iter()
            .zip(&self.ids)
            .map(|(vec, &id)| {
                let dist = self.compute_distance(query, vec);
                (id, dist)
            })
            .collect();

        // Sort by distance ascending
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(scores.into_iter().take(k).collect())
    }

    fn build(&mut self) -> Result<()> {
        Ok(())
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}

/// HNSW Graph Index Stub
/// Placeholder for Hierarchical Navigable Small World Graph
pub struct HNSWIndex {
    flat: FlatIndex,
}

impl HNSWIndex {
    pub fn new(metric: DistanceMetric) -> Self {
        Self {
            flat: FlatIndex::new(metric),
        }
    }
}

impl VectorIndex for HNSWIndex {
    fn add(&mut self, vector: &[f32], row_id: u32) -> Result<()> {
        self.flat.add(vector, row_id)
    }

    fn search(&self, query: &[f32], k: usize) -> Result<Vec<(u32, f32)>> {
        self.flat.search(query, k)
    }

    fn build(&mut self) -> Result<()> {
        self.flat.build()
    }

    fn len(&self) -> usize {
        self.flat.len()
    }
}
