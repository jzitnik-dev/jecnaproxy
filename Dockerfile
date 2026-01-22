FROM rust:1-bookworm as builder

WORKDIR /usr/src/app
COPY . .

# Build release binary
RUN cargo build --release

FROM debian:bookworm-slim

# Install OpenSSL and CA certificates (required for HTTPS)
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/jecnaproxy /usr/local/bin/jecnaproxy

ENV PORT=3000
EXPOSE 3000

CMD ["jecnaproxy"]
