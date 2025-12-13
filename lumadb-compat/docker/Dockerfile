FROM rust:1.75-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --workspace

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/lumadb-server /usr/local/bin/

# Create default config directory
RUN mkdir -p /etc/lumadb && mkdir -p /var/lib/lumadb

COPY config/default.toml /etc/lumadb/config.toml

EXPOSE 5432 3306 9042 27017 9090

VOLUME /var/lib/lumadb

ENTRYPOINT ["lumadb-server"]
CMD ["-c", "/etc/lumadb/config.toml"]
