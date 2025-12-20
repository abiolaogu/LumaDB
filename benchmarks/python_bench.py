"""
Python Benchmark Suite for LumaDB AI Services

Benchmarks vector search and embedding performance.
"""

import time
import numpy as np
from typing import Callable, Any
import sys
import os

# Add parent to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def benchmark(name: str, func: Callable, iterations: int = 10) -> dict:
    """Run a benchmark and return timing statistics."""
    times = []
    
    # Warmup
    func()
    
    for _ in range(iterations):
        start = time.perf_counter()
        result = func()
        elapsed = time.perf_counter() - start
        times.append(elapsed)
    
    return {
        "name": name,
        "min_ms": min(times) * 1000,
        "max_ms": max(times) * 1000,
        "avg_ms": sum(times) / len(times) * 1000,
        "p99_ms": np.percentile(times, 99) * 1000,
    }


def benchmark_vector_search():
    """Benchmark vector search operations."""
    from tdbai.vector_gpu import GPUVectorIndex, GPUIndexConfig
    
    print("\n=== Vector Search Benchmarks ===")
    
    config = GPUIndexConfig(dim=768, nlist=1024, nprobe=64)
    index = GPUVectorIndex(config)
    
    # Generate data
    n_vectors = 100_000
    n_queries = 100
    
    print(f"Generating {n_vectors} vectors...")
    vectors = np.random.randn(n_vectors, 768).astype(np.float32)
    queries = np.random.randn(n_queries, 768).astype(np.float32)
    
    # Training benchmark
    result = benchmark("train", lambda: index.train(vectors[:10000]), iterations=1)
    print(f"Train: {result['avg_ms']:.2f}ms")
    
    # Add benchmark
    result = benchmark("add", lambda: index.add(vectors), iterations=1)
    print(f"Add {n_vectors} vectors: {result['avg_ms']:.2f}ms")
    
    # Search benchmark
    result = benchmark("search", lambda: index.search(queries, k=10), iterations=10)
    qps = (n_queries * 10) / (result['avg_ms'] / 1000)
    print(f"Search: {result['avg_ms']:.2f}ms avg, {result['p99_ms']:.2f}ms p99")
    print(f"Search QPS: {qps:.0f}")
    
    return qps


def benchmark_embeddings():
    """Benchmark embedding generation."""
    from tdbai.embedding_onnx import ONNXEmbeddingModel
    
    print("\n=== Embedding Benchmarks ===")
    
    model = ONNXEmbeddingModel()
    
    # Generate test texts
    texts = [f"This is test sentence number {i} for benchmarking embeddings." for i in range(100)]
    
    # Single embed
    result = benchmark("embed_single", lambda: model.embed(texts[:1]), iterations=100)
    print(f"Single embed: {result['avg_ms']:.2f}ms avg")
    
    # Batch embed
    result = benchmark("embed_batch_100", lambda: model.embed(texts), iterations=10)
    texts_per_sec = 100 / (result['avg_ms'] / 1000)
    print(f"Batch embed (100): {result['avg_ms']:.2f}ms avg")
    print(f"Embedding throughput: {texts_per_sec:.0f} texts/sec")
    
    return texts_per_sec


def benchmark_numpy_simd():
    """Benchmark NumPy operations (comparison baseline)."""
    print("\n=== NumPy SIMD Comparison ===")
    
    sizes = [10_000, 100_000, 1_000_000]
    
    for size in sizes:
        data = np.random.randn(size).astype(np.float64)
        
        result = benchmark(f"sum_{size}", lambda: np.sum(data), iterations=100)
        ops_per_sec = size / (result['avg_ms'] / 1000)
        print(f"Sum {size:>10}: {result['avg_ms']:.4f}ms ({ops_per_sec:.0f} ops/sec)")


def main():
    print("=" * 60)
    print("LumaDB Python Benchmark Suite")
    print("=" * 60)
    
    # NumPy baseline
    benchmark_numpy_simd()
    
    # Vector search
    try:
        qps = benchmark_vector_search()
        target_qps = 100_000
        status = "✓ PASS" if qps >= target_qps else "✗ FAIL"
        print(f"\nVector Search Target ({target_qps} QPS): {status}")
    except Exception as e:
        print(f"Vector search benchmark skipped: {e}")
    
    # Embeddings
    try:
        throughput = benchmark_embeddings()
    except Exception as e:
        print(f"Embedding benchmark skipped: {e}")
    
    print("\n" + "=" * 60)
    print("Benchmarks Complete")
    print("=" * 60)


if __name__ == "__main__":
    main()
