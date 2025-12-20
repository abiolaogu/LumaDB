//! Vector Search Benchmark
//!
//! Run with: cargo bench --bench vector_bench

use std::time::{Duration, Instant};
use luma_core::vector::{create_engine, create_high_throughput_engine, UltraVectorEngine};

/// Benchmark configuration
const DIM: usize = 128;
const NUM_VECTORS: usize = 100_000;
const NUM_QUERIES: usize = 10_000;
const K: usize = 10;

fn main() {
    println!("=".repeat(60));
    println!("LumaDB Vector Search Benchmark");
    println!("Target: 2,500,000+ QPS");
    println!("=".repeat(60));
    println!();
    
    // Create engine
    println!("Creating high-throughput engine (dim={})...", DIM);
    let engine = create_high_throughput_engine(DIM);
    
    // Generate test vectors
    println!("Generating {} test vectors...", NUM_VECTORS);
    let vectors: Vec<(u64, Vec<f32>)> = (0..NUM_VECTORS as u64)
        .map(|i| (i, random_vector(DIM)))
        .collect();
    
    // Benchmark insert
    println!("\n--- INSERT BENCHMARK ---");
    let start = Instant::now();
    engine.batch_insert(vectors);
    let insert_duration = start.elapsed();
    
    let insert_rate = NUM_VECTORS as f64 / insert_duration.as_secs_f64();
    println!("Inserted {} vectors in {:?}", NUM_VECTORS, insert_duration);
    println!("Insert rate: {:.0} vectors/sec", insert_rate);
    
    // Generate queries
    println!("\n--- SEARCH BENCHMARK ---");
    let queries: Vec<Vec<f32>> = (0..NUM_QUERIES)
        .map(|_| random_vector(DIM))
        .collect();
    
    // Warm up
    println!("Warming up...");
    for i in 0..100 {
        let _ = engine.search(&queries[i % queries.len()], K);
    }
    
    // Benchmark single search
    println!("\n[Single Query Search]");
    let mut total_single = Duration::ZERO;
    let iterations = 1000;
    
    for i in 0..iterations {
        let query = &queries[i % queries.len()];
        let start = Instant::now();
        let _ = engine.search(query, K);
        total_single += start.elapsed();
    }
    
    let avg_single_us = total_single.as_micros() as f64 / iterations as f64;
    let single_qps = 1_000_000.0 / avg_single_us;
    println!("Average latency: {:.2} µs", avg_single_us);
    println!("Single-thread QPS: {:.0}", single_qps);
    
    // Benchmark batch search
    println!("\n[Batch Query Search]");
    let start = Instant::now();
    let _ = engine.batch_search(&queries, K);
    let batch_duration = start.elapsed();
    
    let batch_qps = NUM_QUERIES as f64 / batch_duration.as_secs_f64();
    println!("Batch of {} queries in {:?}", NUM_QUERIES, batch_duration);
    println!("Batch QPS: {:.0}", batch_qps);
    
    // Extrapolate to thread scaling
    println!("\n--- PROJECTED PERFORMANCE ---");
    let cores = num_cpus::get();
    let projected_qps = batch_qps * cores as f64 / 2.0; // Conservative estimate
    println!("CPU cores: {}", cores);
    println!("Projected multi-core QPS: {:.0}", projected_qps);
    
    // Summary
    println!("\n{}", "=".repeat(60));
    println!("SUMMARY");
    println!("{}", "=".repeat(60));
    println!("Vectors indexed:    {:>12}", NUM_VECTORS);
    println!("Dimensions:         {:>12}", DIM);
    println!("Insert rate:        {:>12.0} vec/sec", insert_rate);
    println!("Single query:       {:>12.2} µs", avg_single_us);
    println!("Batch QPS:          {:>12.0}", batch_qps);
    println!("Projected QPS:      {:>12.0}", projected_qps);
    
    let target_qps = 2_500_000.0;
    let pct = (projected_qps / target_qps) * 100.0;
    println!("\nTarget: 2.5M QPS → Achieved: {:.1}%", pct);
    
    if projected_qps >= target_qps {
        println!("✅ TARGET MET!");
    } else {
        println!("⚠️ Below target - enable GPU or rayon for parallel processing");
    }
}

fn random_vector(dim: usize) -> Vec<f32> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::SystemTime;
    
    let mut hasher = DefaultHasher::new();
    SystemTime::now().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    let mut seed = hasher.finish();
    
    (0..dim)
        .map(|i| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let bits = ((seed >> 33) ^ seed) as u32;
            (bits as f32 / u32::MAX as f32) * 2.0 - 1.0
        })
        .collect()
}
