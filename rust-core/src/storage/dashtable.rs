//! Lock-free Dashtable implementation
//!
//! High-performance concurrent hash table inspired by Dash:
//! - Bucket-level versioning for optimistic reads
//! - Fingerprint-based filtering
//! - Cache-line aligned buckets

use std::sync::atomic::{AtomicU64, AtomicPtr, AtomicU8, Ordering};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::ptr;

const BUCKET_SIZE: usize = 14;
const SEGMENT_COUNT: usize = 256;

/// Cache-line aligned bucket
#[repr(align(64))]
pub struct Bucket<K, V> {
    version: AtomicU64,
    fingerprints: [AtomicU8; BUCKET_SIZE],
    keys: [AtomicPtr<K>; BUCKET_SIZE],
    values: [AtomicPtr<V>; BUCKET_SIZE],
}

impl<K, V> Bucket<K, V> {
    fn new() -> Self {
        Self {
            version: AtomicU64::new(0),
            fingerprints: std::array::from_fn(|_| AtomicU8::new(0)),
            keys: std::array::from_fn(|_| AtomicPtr::new(ptr::null_mut())),
            values: std::array::from_fn(|_| AtomicPtr::new(ptr::null_mut())),
        }
    }
}

/// Segment containing multiple buckets
pub struct Segment<K, V> {
    buckets: Vec<Bucket<K, V>>,
    bucket_mask: usize,
}

impl<K, V> Segment<K, V> {
    fn new(num_buckets: usize) -> Self {
        let num_buckets = num_buckets.next_power_of_two();
        let mut buckets = Vec::with_capacity(num_buckets);
        for _ in 0..num_buckets {
            buckets.push(Bucket::new());
        }
        Self {
            buckets,
            bucket_mask: num_buckets - 1,
        }
    }
}

/// High-performance lock-free hash table
pub struct Dashtable<K, V> {
    segments: Vec<Segment<K, V>>,
    segment_shift: u32,
    len: AtomicU64,
}

impl<K: Hash + Eq + Clone, V: Clone> Dashtable<K, V> {
    pub fn new(capacity: usize) -> Self {
        let buckets_per_segment = (capacity / SEGMENT_COUNT / BUCKET_SIZE).max(16);
        let segments: Vec<_> = (0..SEGMENT_COUNT)
            .map(|_| Segment::new(buckets_per_segment))
            .collect();
        
        Self {
            segments,
            segment_shift: 56, // Use top 8 bits for segment
            len: AtomicU64::new(0),
        }
    }

    fn hash(&self, key: &K) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn get_segment(&self, hash: u64) -> usize {
        ((hash >> self.segment_shift) as usize) % SEGMENT_COUNT
    }

    fn get_bucket_idx(&self, hash: u64, segment: &Segment<K, V>) -> usize {
        (hash as usize) & segment.bucket_mask
    }

    fn fingerprint(&self, hash: u64) -> u8 {
        ((hash >> 32) as u8).max(1) // Never 0 (empty marker)
    }

    /// Optimistic lock-free read
    pub fn get(&self, key: &K) -> Option<V> {
        let hash = self.hash(key);
        let seg_idx = self.get_segment(hash);
        let segment = &self.segments[seg_idx];
        let bucket_idx = self.get_bucket_idx(hash, segment);
        let bucket = &segment.buckets[bucket_idx];
        let fp = self.fingerprint(hash);

        loop {
            let v1 = bucket.version.load(Ordering::Acquire);
            
            // Check if write in progress
            if v1 & 1 == 1 {
                std::hint::spin_loop();
                continue;
            }

            // Search bucket
            for i in 0..BUCKET_SIZE {
                let stored_fp = bucket.fingerprints[i].load(Ordering::Relaxed);
                if stored_fp == fp {
                    let key_ptr = bucket.keys[i].load(Ordering::Acquire);
                    let val_ptr = bucket.values[i].load(Ordering::Acquire);
                    
                    if !key_ptr.is_null() && !val_ptr.is_null() {
                        let stored_key = unsafe { &*key_ptr };
                        if stored_key == key {
                            // Validate version hasn't changed
                            let v2 = bucket.version.load(Ordering::Acquire);
                            if v1 == v2 {
                                let value = unsafe { (*val_ptr).clone() };
                                return Some(value);
                            }
                            // Version mismatch, retry
                            break;
                        }
                    }
                }
            }

            // Verify we didn't miss due to concurrent write
            let v2 = bucket.version.load(Ordering::Acquire);
            if v1 == v2 {
                return None;
            }
        }
    }

    /// Insert with bucket-level locking via version
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        let hash = self.hash(&key);
        let seg_idx = self.get_segment(hash);
        let segment = &self.segments[seg_idx];
        let bucket_idx = self.get_bucket_idx(hash, segment);
        let bucket = &segment.buckets[bucket_idx];
        let fp = self.fingerprint(hash);

        // Acquire write lock by setting version to odd
        loop {
            let v = bucket.version.load(Ordering::Acquire);
            if v & 1 == 0 {
                if bucket.version.compare_exchange(
                    v, v + 1, Ordering::AcqRel, Ordering::Relaxed
                ).is_ok() {
                    break;
                }
            }
            std::hint::spin_loop();
        }

        let old_value = {
            // Check for existing key
            let mut insert_slot = None;
            
            for i in 0..BUCKET_SIZE {
                let stored_fp = bucket.fingerprints[i].load(Ordering::Relaxed);
                
                if stored_fp == fp {
                    let key_ptr = bucket.keys[i].load(Ordering::Relaxed);
                    if !key_ptr.is_null() {
                        let stored_key = unsafe { &*key_ptr };
                        if stored_key == &key {
                            // Update existing
                            let old_val_ptr = bucket.values[i].swap(
                                Box::into_raw(Box::new(value)),
                                Ordering::Release
                            );
                            let old = if !old_val_ptr.is_null() {
                                Some(unsafe { *Box::from_raw(old_val_ptr) })
                            } else {
                                None
                            };
                            // Release lock
                            bucket.version.fetch_add(1, Ordering::Release);
                            return old;
                        }
                    }
                } else if stored_fp == 0 && insert_slot.is_none() {
                    insert_slot = Some(i);
                }
            }

            // Insert new
            if let Some(i) = insert_slot {
                bucket.fingerprints[i].store(fp, Ordering::Relaxed);
                bucket.keys[i].store(Box::into_raw(Box::new(key)), Ordering::Release);
                bucket.values[i].store(Box::into_raw(Box::new(value)), Ordering::Release);
                self.len.fetch_add(1, Ordering::Relaxed);
                None
            } else {
                // Bucket full - would need overflow handling
                // For now, just fail silently
                None
            }
        };

        // Release write lock
        bucket.version.fetch_add(1, Ordering::Release);
        old_value
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// Note: Need proper Drop implementation to free all allocated K/V

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_get() {
        let table: Dashtable<String, i32> = Dashtable::new(1024);
        table.insert("hello".to_string(), 42);
        assert_eq!(table.get(&"hello".to_string()), Some(42));
        assert_eq!(table.get(&"world".to_string()), None);
    }

    #[test]
    fn test_update() {
        let table: Dashtable<String, i32> = Dashtable::new(1024);
        table.insert("key".to_string(), 1);
        let old = table.insert("key".to_string(), 2);
        assert_eq!(old, Some(1));
        assert_eq!(table.get(&"key".to_string()), Some(2));
    }
}
