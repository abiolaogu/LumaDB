
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// A simplified HyperLogLog implementation for cardinality estimation.
pub struct HLLSketch {
    registers: Vec<u8>,
    p: u8,
    m: usize,
}

impl HLLSketch {
    /// Create new HLL with standard precision p=14 (error ~0.8%)
    pub fn new() -> Self {
        let p = 14;
        let m = 1 << p;
        Self {
            registers: vec![0; m],
            p,
            m,
        }
    }

    pub fn add(&mut self, item: impl Hash) {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        let x = hasher.finish();
        
        let j = x >> (64 - self.p); // First p bits idx
        let w = x << self.p; // Remaining bits
        let rho = w.leading_zeros() as u8 + 1;
        
        let idx = j as usize;
        if rho > self.registers[idx] {
            self.registers[idx] = rho;
        }
    }

    pub fn count(&self) -> u64 {
        let alpha = 0.7213 / (1.0 + 1.079 / (self.m as f64));
        let sum: f64 = self.registers.iter()
            .map(|&val| 2.0f64.powi(-(val as i32)))
            .sum();
        
        let estimate = alpha * (self.m as f64).powi(2) / sum;
        
        if estimate <= 2.5 * (self.m as f64) {
            let zeros = self.registers.iter().filter(|&&val| val == 0).count();
            if zeros > 0 {
                (self.m as f64 * (self.m as f64 / zeros as f64).ln()) as u64
            } else {
                estimate as u64
            }
        } else {
            estimate as u64
        }
    }
}
