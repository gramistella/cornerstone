# --- Stage 1: Builder ---
# Combines incremental Rust builds with efficient Node.js caching.
FROM rust:1.88.0 AS builder
WORKDIR /app

# Declare a build-time argument for the database URL
ARG DATABASE_URL_ARG

# 2. Set the ENV variable for this stage using the value from the ARG
ENV DATABASE_URL=${DATABASE_URL_ARG}

# Install system dependencies needed for both frontend and backend
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev nodejs npm && \
    rm -rf /var/lib/apt/lists/*

# Install sqlx-cli for migrations
RUN cargo install sqlx-cli --no-default-features --features native-tls,sqlite

## --- Frontend Build (Optimized Caching) ---

# 1. Copy ONLY the package manifests first to cache the npm install layer
COPY frontend_svelte/package.json frontend_svelte/package-lock.json* ./frontend_svelte/

# 2. Install dependencies. This layer is only re-run if the manifest files change.
#    We use --prefix to specify the working directory for npm.
RUN --mount=type=cache,target=/root/.npm \
    npm --prefix frontend_svelte install

# 3. Now, copy the rest of the project source code
COPY . .

# 4. Build the Svelte frontend using the previously installed node_modules
RUN npm --prefix frontend_svelte run build

## --- Backend Build (Incremental) ---

# 5. Create the database and run migrations. This is required for sqlx::migrate!
RUN sqlx database create && \
    sqlx migrate run --source backend/migrations

# 6. Copy the built frontend into the backend's static directory.
RUN mkdir -p backend/static/svelte-build && \
    cp -r frontend_svelte/build/* backend/static/svelte-build/

# 7. Build the Rust backend with cache mounts for incremental compilation.
#    This is the most efficient method for iterative development.
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    sh -c ' \
        cargo build --release --package backend --features svelte-ui && \
        mkdir -p /app/artifacts && \
        cp /app/target/release/backend /app/artifacts/ \
    '

# --- Stage 2: Runtime ---
# This is the final, minimal image for production (unchanged).
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy only the necessary artifacts from the 'builder' stage.
COPY --from=builder /app/artifacts/backend ./cornerstone-server
COPY --from=builder /app/backend/static/svelte-build ./backend/static/svelte-build
COPY --from=builder /app/backend/database.db ./backend/database.db
COPY --from=builder /app/Config.toml .

EXPOSE 8080
ENV APP_DATABASE__URL="sqlite:/app/backend/database.db"
ENV RUST_LOG="info"

CMD ["./cornerstone-server"]