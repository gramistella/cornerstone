# --- Stage 1: Builder ---
# Combines incremental Rust builds with efficient Node.js caching.
FROM rust:1.88.0 AS builder
WORKDIR /app

# Declare build-time arguments
ARG DATABASE_URL_ARG
ARG UI_TYPE_ARG
ARG DB_TYPE_ARG

# Set the ENV variable for this stage using the value from the ARG
ENV DATABASE_URL=${DATABASE_URL_ARG}

# Install system dependencies. We install node/npm unconditionally for simplicity.
RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev nodejs npm && \
    rm -rf /var/lib/apt/lists/*

# Install sqlx-cli for migrations with support for BOTH database types
RUN cargo install sqlx-cli --no-default-features --features native-tls,sqlite,postgres

## --- Frontend Build (Optimized Caching) ---

# 1. Copy ONLY the package manifests first to cache the npm install layer
COPY frontend_svelte/package.json frontend_svelte/package-lock.json* ./frontend_svelte/
RUN --mount=type=cache,target=/root/.npm \
    npm --prefix frontend_svelte install

# 2. Now, copy the rest of the project source code
COPY . .

# 3. This single, dynamic build step handles everything else.
RUN --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/usr/local/cargo/registry \
    sh -c ' \
        # 1. Determine DB and UI type automatically (using POSIX-compliant commands) \
        if [ -n "$UI_TYPE_ARG" ] && [ -n "$DB_TYPE_ARG" ]; then \
            UI_TYPE="$UI_TYPE_ARG"; \
            DB_TYPE="$DB_TYPE_ARG"; \
            echo "==> Using provided configuration: UI=$UI_TYPE, DB=$DB_TYPE"; \
        else \
            echo "==> Detecting configuration..." && \
            HELPER_OUTPUT=$(cargo run --quiet -p common --bin workspace_helper) && \
            DB_TYPE=$(echo "$HELPER_OUTPUT" | cut -d" " -f1) && \
            UI_TYPE=$(echo "$HELPER_OUTPUT" | cut -d" " -f2) && \
            echo "==> Detected configuration: UI=$UI_TYPE, DB=$DB_TYPE"; \
        fi && \
        \
        # 2. Conditionally build the frontend \
        if [ "$UI_TYPE" = "svelte" ]; then \
            echo "==> Building Svelte frontend..." && \
            npm --prefix frontend_svelte run build && \
            echo "==> Copying Svelte assets..." && \
            mkdir -p backend/static/svelte-build && \
            cp -r frontend_svelte/build/* backend/static/svelte-build/; \
        elif [ "$UI_TYPE" = "slint" ]; then \
            echo "==> Installing wasm-pack and building Slint frontend..." && \
            cargo install wasm-pack && \
            cd frontend_slint && wasm-pack build --target web --out-dir static/wasm-pack && cd .. && \
            echo "==> Cleaning and copying Slint assets..." && \
            find backend/static/slint-build -maxdepth 1 -type f ! -name "index.html" -delete && \
            mkdir -p backend/static/slint-build/wasm && \
            find backend/static/slint-build/wasm -mindepth 1 ! -name ".gitkeep" -exec rm -rf {} + && \
            cp -r frontend_slint/static/wasm-pack/* backend/static/slint-build/wasm/ && \
            rm -f backend/static/slint-build/wasm/index.html; \
        fi && \
        \
        # 3. Create database and run migrations \
        echo "==> Creating database and running migrations for $DB_TYPE..." && \
        touch backend/database.db && \
        sqlx database create && \
        sqlx migrate run --source backend/migrations/$DB_TYPE && \
        \
        # 4. Build the backend with the correct features \
        echo "==> Building backend application..." && \
        cargo build --release --package backend --no-default-features --features "${UI_TYPE}-ui,db-${DB_TYPE}" && \
        \
        # 5. Copy the final artifact \
        echo "==> Copying artifact..." && \
        mkdir -p /app/artifacts && \
        cp /app/target/release/backend /app/artifacts/ \
    '

# --- Stage 2: Runtime ---
# This is the final, minimal image for production.
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy only the necessary artifacts from the 'builder' stage.
COPY --from=builder /app/artifacts/backend ./cornerstone-server
# This now copies the entire static folder, which will contain the correct UI sub-folder
COPY --from=builder /app/backend/static ./backend/static
# This will copy the database file if sqlite was used, or an empty file otherwise (which is harmless)
COPY --from=builder /app/backend/database.db ./backend/database.db
COPY --from=builder /app/Config.toml .

EXPOSE 8080

ENV RUST_LOG="info"

CMD ["./cornerstone-server"]
