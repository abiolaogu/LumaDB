"""
GPU-Accelerated FAISS Vector Index

High-performance vector search using FAISS with GPU acceleration.
Supports billion-scale vector search with IVF-PQ indexing.
"""

import numpy as np
from typing import List, Tuple, Optional
from dataclasses import dataclass
import threading

try:
    import faiss
    FAISS_AVAILABLE = True
except ImportError:
    FAISS_AVAILABLE = False
    print("Warning: FAISS not available, using mock implementation")


@dataclass
class GPUIndexConfig:
    """Configuration for GPU vector index."""
    dim: int
    nlist: int = 16384          # Number of IVF clusters
    m: int = 64                  # Subquantizers for PQ
    nbits: int = 8               # Bits per subquantizer
    nprobe: int = 128            # Clusters to search
    use_float16: bool = True     # GPU memory optimization
    num_gpus: int = 1


class GPUVectorIndex:
    """High-performance GPU-accelerated vector index using FAISS."""

    def __init__(self, config: GPUIndexConfig):
        self.config = config
        self.dim = config.dim
        self._lock = threading.RLock()
        self.is_trained = False
        
        if not FAISS_AVAILABLE:
            self._mock_vectors = []
            return

        # Initialize GPU resources
        self.gpu_resources = []
        for i in range(config.num_gpus):
            res = faiss.StandardGpuResources()
            res.setTempMemory(1024 * 1024 * 1024)  # 1GB temp memory
            self.gpu_resources.append(res)

        # Build IVF-PQ index (optimal for billion-scale)
        quantizer = faiss.IndexFlatL2(config.dim)
        self.cpu_index = faiss.IndexIVFPQ(
            quantizer,
            config.dim,
            config.nlist,
            config.m,
            config.nbits
        )

        # GPU cloner options
        self.co = faiss.GpuMultipleClonerOptions()
        self.co.shard = True
        self.co.useFloat16 = config.use_float16

        self.gpu_index = None

    def train(self, vectors: np.ndarray) -> None:
        """Train the index on a sample of vectors."""
        with self._lock:
            if not FAISS_AVAILABLE:
                self.is_trained = True
                return

            if vectors.dtype != np.float32:
                vectors = vectors.astype(np.float32)

            faiss.normalize_L2(vectors)
            self.cpu_index.train(vectors)
            self._sync_to_gpu()
            self.is_trained = True

    def add(self, vectors: np.ndarray, ids: Optional[np.ndarray] = None) -> None:
        """Add vectors to the index."""
        with self._lock:
            if not self.is_trained:
                raise ValueError("Index must be trained before adding vectors")

            if not FAISS_AVAILABLE:
                self._mock_vectors.extend(vectors.tolist())
                return

            if vectors.dtype != np.float32:
                vectors = vectors.astype(np.float32)

            faiss.normalize_L2(vectors)

            if ids is not None:
                self.cpu_index.add_with_ids(vectors, ids)
            else:
                self.cpu_index.add(vectors)

            self._sync_to_gpu()

    def search(
        self,
        queries: np.ndarray,
        k: int = 10,
        nprobe: Optional[int] = None
    ) -> Tuple[np.ndarray, np.ndarray]:
        """Search for k nearest neighbors."""
        if not FAISS_AVAILABLE:
            # Mock implementation
            n = queries.shape[0]
            return np.zeros((n, k)), np.zeros((n, k), dtype=np.int64)

        if queries.dtype != np.float32:
            queries = queries.astype(np.float32)

        faiss.normalize_L2(queries)

        if nprobe is None:
            nprobe = self.config.nprobe

        self.gpu_index.nprobe = nprobe
        distances, indices = self.gpu_index.search(queries, k)

        return distances, indices

    def search_with_filter(
        self,
        queries: np.ndarray,
        k: int,
        filter_ids: np.ndarray
    ) -> Tuple[np.ndarray, np.ndarray]:
        """Search with ID filter (post-filtering)."""
        # Over-fetch then filter
        distances, indices = self.search(queries, k * 10)

        # Apply filter
        mask = np.isin(indices, filter_ids)
        
        # For each query, take first k valid results
        n_queries = queries.shape[0]
        filtered_distances = np.full((n_queries, k), np.inf)
        filtered_indices = np.full((n_queries, k), -1, dtype=np.int64)
        
        for i in range(n_queries):
            valid_mask = mask[i]
            valid_dists = distances[i][valid_mask]
            valid_ids = indices[i][valid_mask]
            
            n_valid = min(len(valid_dists), k)
            filtered_distances[i, :n_valid] = valid_dists[:n_valid]
            filtered_indices[i, :n_valid] = valid_ids[:n_valid]

        return filtered_distances, filtered_indices

    def _sync_to_gpu(self) -> None:
        """Sync CPU index to GPU."""
        if FAISS_AVAILABLE and self.gpu_resources:
            self.gpu_index = faiss.index_cpu_to_gpu_multiple_py(
                self.gpu_resources,
                self.cpu_index,
                self.co
            )

    def save(self, path: str) -> None:
        """Save index to disk."""
        if FAISS_AVAILABLE:
            faiss.write_index(self.cpu_index, path)

    @classmethod
    def load(cls, path: str, config: GPUIndexConfig) -> 'GPUVectorIndex':
        """Load index from disk."""
        index = cls(config)
        if FAISS_AVAILABLE:
            index.cpu_index = faiss.read_index(path)
            index._sync_to_gpu()
            index.is_trained = True
        return index

    def __len__(self) -> int:
        """Return number of vectors in index."""
        if not FAISS_AVAILABLE:
            return len(self._mock_vectors)
        return self.cpu_index.ntotal
