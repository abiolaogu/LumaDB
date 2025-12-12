package federation

import (
	"errors"
	"sync"
)

var ErrSourceNotFound = errors.New("source not found")

// SourceRegistry manages federated data sources
type SourceRegistry struct {
	mu      sync.RWMutex
	sources map[string]Source
}

func NewSourceRegistry() *SourceRegistry {
	return &SourceRegistry{
		sources: make(map[string]Source),
	}
}

func (r *SourceRegistry) Register(name string, source Source) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.sources[name] = source
}

func (r *SourceRegistry) Get(name string) (Source, error) {
	r.mu.RLock()
	defer r.mu.RUnlock()
	s, ok := r.sources[name]
	if !ok {
		return nil, ErrSourceNotFound
	}
	return s, nil
}

func (r *SourceRegistry) List() map[string]Source {
	r.mu.RLock()
	defer r.mu.RUnlock()

	// Copy to avoid race
	copy := make(map[string]Source)
	for k, v := range r.sources {
		copy[k] = v
	}
	return copy
}
