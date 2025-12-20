// Package cluster implements pipelined Raft transport
// for non-blocking message delivery.
package cluster

import (
	"sync"
)

// RaftMessage represents a Raft protocol message
type RaftMessage struct {
	From    uint64
	To      uint64
	GroupID uint64
	Type    string
	Data    []byte
}

// PipelineTransport sends Raft messages without waiting for responses
type PipelineTransport struct {
	mu       sync.RWMutex
	streams  map[uint64]chan RaftMessage
	inflight *InflightTracker
}

// InflightTracker tracks messages in flight
type InflightTracker struct {
	mu      sync.Mutex
	pending map[uint64]int // nodeID -> count
	limit   int
}

func NewInflightTracker(limit int) *InflightTracker {
	return &InflightTracker{
		pending: make(map[uint64]int),
		limit:   limit,
	}
}

func (t *InflightTracker) Acquire(nodeID uint64) bool {
	t.mu.Lock()
	defer t.mu.Unlock()
	if t.pending[nodeID] >= t.limit {
		return false
	}
	t.pending[nodeID]++
	return true
}

func (t *InflightTracker) Release(nodeID uint64) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if t.pending[nodeID] > 0 {
		t.pending[nodeID]--
	}
}

// NewPipelineTransport creates a new pipelined transport
func NewPipelineTransport() *PipelineTransport {
	return &PipelineTransport{
		streams:  make(map[uint64]chan RaftMessage),
		inflight: NewInflightTracker(100),
	}
}

// Connect creates a stream to a peer
func (t *PipelineTransport) Connect(nodeID uint64) {
	t.mu.Lock()
	defer t.mu.Unlock()
	if _, exists := t.streams[nodeID]; !exists {
		t.streams[nodeID] = make(chan RaftMessage, 1000)
	}
}

// Send queues a message for delivery (non-blocking)
func (t *PipelineTransport) Send(msg RaftMessage) error {
	t.mu.RLock()
	stream, exists := t.streams[msg.To]
	t.mu.RUnlock()

	if !exists {
		// Auto-connect
		t.Connect(msg.To)
		t.mu.RLock()
		stream = t.streams[msg.To]
		t.mu.RUnlock()
	}

	// Non-blocking send with flow control
	if !t.inflight.Acquire(msg.To) {
		// Backpressure: drop or block
		return nil
	}

	select {
	case stream <- msg:
		return nil
	default:
		t.inflight.Release(msg.To)
		// Channel full, drop message (in real impl: buffer or retry)
		return nil
	}
}

// Pipeline sends multiple messages without waiting
func (t *PipelineTransport) Pipeline(msgs []RaftMessage) error {
	for _, msg := range msgs {
		if err := t.Send(msg); err != nil {
			return err
		}
	}
	return nil
}

// Receive gets messages for a node (for testing/local delivery)
func (t *PipelineTransport) Receive(nodeID uint64) <-chan RaftMessage {
	t.mu.RLock()
	defer t.mu.RUnlock()
	return t.streams[nodeID]
}
