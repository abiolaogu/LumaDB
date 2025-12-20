// Package api implements zero-copy gRPC optimizations
package api

import (
	"sync"

	"google.golang.org/protobuf/proto"
)

// BufferPool for reusing byte slices
var bufferPool = sync.Pool{
	New: func() interface{} {
		buf := make([]byte, 0, 64*1024)
		return &buf
	},
}

// GetBuffer gets a buffer from the pool
func GetBuffer() *[]byte {
	return bufferPool.Get().(*[]byte)
}

// PutBuffer returns a buffer to the pool
func PutBuffer(buf *[]byte) {
	*buf = (*buf)[:0] // Reset length but keep capacity
	bufferPool.Put(buf)
}

// ZeroCopyCodec minimizes allocations for gRPC
type ZeroCopyCodec struct{}

// Marshal serializes a protobuf message with buffer reuse
func (c *ZeroCopyCodec) Marshal(v interface{}) ([]byte, error) {
	msg, ok := v.(proto.Message)
	if !ok {
		return nil, nil
	}

	size := proto.Size(msg)
	bufPtr := GetBuffer()
	buf := *bufPtr

	if cap(buf) < size {
		buf = make([]byte, 0, size)
	}

	result, err := proto.MarshalOptions{}.MarshalAppend(buf[:0], msg)
	if err != nil {
		PutBuffer(bufPtr)
		return nil, err
	}

	return result, nil
}

// Unmarshal deserializes a protobuf message
func (c *ZeroCopyCodec) Unmarshal(data []byte, v interface{}) error {
	msg, ok := v.(proto.Message)
	if !ok {
		return nil
	}
	return proto.Unmarshal(data, msg)
}

// Name returns the codec name
func (c *ZeroCopyCodec) Name() string {
	return "zerocopy"
}

// ReturnBuffer returns buffer to pool after use
func ReturnBuffer(buf []byte) {
	bufPtr := &buf
	PutBuffer(bufPtr)
}
