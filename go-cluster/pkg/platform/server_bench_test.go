package platform

import (
	"testing"

	"github.com/lumadb/cluster/pkg/platform/auth"
	"github.com/valyala/fasthttp"
)

// BenchmarkHello benchmarks the raw HTTP throughput
func BenchmarkHello(b *testing.B) {
	// Simple handler that mimics the structure of our app handler
	handler := func(ctx *fasthttp.RequestCtx) {
		ctx.SetContentType("application/json")
		ctx.SetStatusCode(fasthttp.StatusOK)
		ctx.SetBodyString(`{"status":"ok"}`)
	}

	ctx := &fasthttp.RequestCtx{}

	b.ResetTimer()
	b.ReportAllocs()

	for i := 0; i < b.N; i++ {
		handler(ctx)
	}
}

// Mock structures to satisfy dependencies if we were to instantiate real server
// Kept simple for now as we are benchmarking handler logic primarily.
type mockAuthEngine struct{}

func (m *mockAuthEngine) IsAuthorized(role string, action auth.Action) bool { return true }
func (m *mockAuthEngine) GenerateToken(user, role string) (string, error)   { return "token", nil }
func (m *mockAuthEngine) ValidateToken(token string) (*auth.Claims, error) {
	return &auth.Claims{}, nil
}
