package benchmarks

import (
	"sync"
	"testing"
)

// BenchmarkParallelRaft benchmarks parallel Raft ticking
func BenchmarkParallelRaft(b *testing.B) {
	// Would setup actual ParallelRaftEngine here
	// engine := cluster.NewParallelRaftEngine(zap.NewNop(), 100*time.Millisecond)
	// for i := uint64(0); i < 100; i++ {
	// 	engine.AddGroup(i)
	// }

	b.ResetTimer()
	b.RunParallel(func(pb *testing.PB) {
		for pb.Next() {
			// engine.Tick(context.Background())
		}
	})
}

// BenchmarkConcurrentMap benchmarks concurrent map access
func BenchmarkConcurrentMap(b *testing.B) {
	m := sync.Map{}

	// Pre-populate
	for i := 0; i < 10000; i++ {
		m.Store(i, i)
	}

	b.ResetTimer()
	b.RunParallel(func(pb *testing.PB) {
		i := 0
		for pb.Next() {
			if i%2 == 0 {
				m.Load(i % 10000)
			} else {
				m.Store(i%10000, i)
			}
			i++
		}
	})
}

// BenchmarkChannelThroughput benchmarks channel throughput
func BenchmarkChannelThroughput(b *testing.B) {
	ch := make(chan int, 1000)

	go func() {
		for v := range ch {
			_ = v
		}
	}()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		ch <- i
	}
	close(ch)
}

// BenchmarkBufferPool benchmarks buffer pool performance
func BenchmarkBufferPool(b *testing.B) {
	pool := sync.Pool{
		New: func() interface{} {
			buf := make([]byte, 64*1024)
			return &buf
		},
	}

	b.ResetTimer()
	b.RunParallel(func(pb *testing.PB) {
		for pb.Next() {
			buf := pool.Get().(*[]byte)
			// Simulate work
			(*buf)[0] = 1
			pool.Put(buf)
		}
	})
}

// BenchmarkSumInt64Go benchmarks pure Go sum (for comparison with Rust FFI)
func BenchmarkSumInt64Go(b *testing.B) {
	data := make([]int64, 1000000)
	for i := range data {
		data[i] = int64(i)
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		sum := int64(0)
		for _, v := range data {
			sum += v
		}
		_ = sum
	}

	b.ReportMetric(float64(len(data))/float64(b.Elapsed().Nanoseconds())*1e9, "elements/sec")
}
