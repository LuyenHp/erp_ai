# =============================================================================
# Stage 1: Builder – Compile Rust binary
# =============================================================================
FROM rust:1.88-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to cache dependency compilation
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src/ src/

# Build the real binary (touch main.rs to invalidate cache for our code only)
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 2: Runtime – Minimal image with only the binary
# =============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r appuser && useradd -r -g appuser -s /sbin/nologin appuser

WORKDIR /app

# Copy only the compiled binary from builder
COPY --from=builder /app/target/release/erp_ai .

# Copy migrations for runtime execution
COPY migrations/ migrations/

# Switch to non-root user
USER appuser

EXPOSE 3000

CMD ["./erp_ai"]
