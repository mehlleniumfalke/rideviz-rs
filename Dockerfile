# Frontend build stage
FROM node:22-slim as frontend-builder

WORKDIR /frontend

COPY rideviz-web/package.json rideviz-web/package-lock.json ./
RUN npm ci

COPY rideviz-web/ ./
RUN npm run build

# Build stage
FROM rust:1.85-slim as builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY . .

# Build the actual application (touch src to force cargo to relink after dummy build)
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    fonts-dejavu-core \
    && mkdir -p /app/assets/fonts \
    && curl -fsSL "https://raw.githubusercontent.com/vercel/geist-font/main/fonts/Geist/otf/Geist-Regular.otf" -o /app/assets/fonts/Geist-Regular.otf \
    && curl -fsSL "https://raw.githubusercontent.com/vercel/geist-font/main/fonts/GeistPixel/ttf/GeistPixel-Square.ttf" -o /app/assets/fonts/GeistPixel-Square.ttf \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /build/target/release/rideviz-rs /app/rideviz-rs

# Copy frontend from frontend-builder
COPY --from=frontend-builder /frontend/dist /app/assets/web

# Create non-root user
RUN useradd -m -u 1001 rideviz && \
    chown -R rideviz:rideviz /app

USER rideviz

EXPOSE 3000

CMD ["/app/rideviz-rs"]
