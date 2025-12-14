
# LumaDB Single Binary Dockerfile
FROM rust:1.75 AS builder

WORKDIR /usr/src/lumadb
COPY . .

# Build with optimizations
RUN cargo build --release --package luma-server

# Runtime Stage (Distroless for minimal size)
FROM gcr.io/distroless/cc-debian12

COPY --from=builder /usr/src/lumadb/target/release/luma-server /usr/local/bin/luma-server

# Ports: 8080 (HTTP), 50051 (gRPC), 5432 (PG), 9090 (Prom), 4317 (OTLP)
EXPOSE 8080 50051 5432 9090 4317

ENTRYPOINT ["/usr/local/bin/luma-server"]
