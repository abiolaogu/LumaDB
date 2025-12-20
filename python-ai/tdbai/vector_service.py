"""
Vector Search FastAPI Service

Provides REST API endpoints for vector search operations.
"""

from typing import List, Optional
from pydantic import BaseModel
import numpy as np
import time

try:
    from fastapi import FastAPI, HTTPException
    FASTAPI_AVAILABLE = True
except ImportError:
    FASTAPI_AVAILABLE = False

from .vector_gpu import GPUVectorIndex, GPUIndexConfig


class SearchRequest(BaseModel):
    """Vector search request."""
    vectors: List[List[float]]
    k: int = 10
    nprobe: int = 128
    filter_ids: Optional[List[int]] = None


class SearchResponse(BaseModel):
    """Vector search response."""
    distances: List[List[float]]
    indices: List[List[int]]
    latency_ms: float


class AddRequest(BaseModel):
    """Add vectors request."""
    vectors: List[List[float]]
    ids: Optional[List[int]] = None


class AddResponse(BaseModel):
    """Add vectors response."""
    count: int
    latency_ms: float


# Global index instance
_index: Optional[GPUVectorIndex] = None


def get_index() -> GPUVectorIndex:
    """Get or create the global index."""
    global _index
    if _index is None:
        config = GPUIndexConfig(dim=768)  # Default dimension
        _index = GPUVectorIndex(config)
    return _index


def create_app() -> "FastAPI":
    """Create FastAPI application."""
    if not FASTAPI_AVAILABLE:
        raise ImportError("FastAPI not available")
    
    app = FastAPI(title="LumaDB Vector Search", version="1.0.0")

    @app.post("/search", response_model=SearchResponse)
    async def search(request: SearchRequest) -> SearchResponse:
        """Search for nearest neighbors."""
        start = time.perf_counter()
        
        index = get_index()
        queries = np.array(request.vectors, dtype=np.float32)
        
        if request.filter_ids:
            distances, indices = index.search_with_filter(
                queries, request.k, np.array(request.filter_ids)
            )
        else:
            distances, indices = index.search(queries, request.k, request.nprobe)
        
        latency = (time.perf_counter() - start) * 1000
        
        return SearchResponse(
            distances=distances.tolist(),
            indices=indices.tolist(),
            latency_ms=latency
        )

    @app.post("/add", response_model=AddResponse)
    async def add(request: AddRequest) -> AddResponse:
        """Add vectors to the index."""
        start = time.perf_counter()
        
        index = get_index()
        vectors = np.array(request.vectors, dtype=np.float32)
        ids = np.array(request.ids) if request.ids else None
        
        index.add(vectors, ids)
        
        latency = (time.perf_counter() - start) * 1000
        
        return AddResponse(count=len(request.vectors), latency_ms=latency)

    @app.post("/train")
    async def train(request: AddRequest):
        """Train the index on sample vectors."""
        start = time.perf_counter()
        
        index = get_index()
        vectors = np.array(request.vectors, dtype=np.float32)
        index.train(vectors)
        
        latency = (time.perf_counter() - start) * 1000
        
        return {"status": "trained", "latency_ms": latency}

    @app.get("/health")
    async def health():
        """Health check endpoint."""
        return {"status": "ok", "vectors": len(get_index())}

    return app


# Create app if FastAPI available
if FASTAPI_AVAILABLE:
    app = create_app()
