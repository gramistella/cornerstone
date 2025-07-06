# --- Stage 1: Planner ---
# This stage calculates the dependency plan. It's small and fast.
FROM docker.io/lukemathwalker/cargo-chef:latest-rust-1.88.0 AS planner
WORKDIR /app

# We need `just` to run our build scripts
RUN cargo install just

# Copy all Cargo manifests and the lock file
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./justfile ./justfile

# Copy the source code for ALL workspace crates
# This is crucial for cargo-chef to understand the full dependency tree
COPY ./common/src ./common/src
COPY ./common/Cargo.toml ./common/Cargo.toml
COPY ./backend/src ./backend/src
COPY ./backend/Cargo.toml ./backend/Cargo.toml
# Always copy the frontend Slint crate, even if it's not used in the backend to make cargo-chef happy
COPY ./frontend_slint/src ./frontend_slint/src
COPY ./frontend_slint/ui ./frontend_slint/ui
COPY ./frontend_slint/Cargo.toml ./frontend_slint/Cargo.toml
# If you add more Rust crates to the workspace, copy them here too

# Generate the recipe file
RUN cargo chef prepare --recipe-path recipe.json

# --- Stage 2: Cacher ---
# This stage builds and caches the dependencies. It will only re-run
# if the recipe.json (and thus Cargo.lock) has changed.
FROM docker.io/lukemathwalker/cargo-chef:latest-rust-1.88.0 AS cacher
WORKDIR /app
RUN cargo install just

# Make sure to change this if you want to use Postgres
RUN cargo install sqlx-cli --no-default-features --features native-tls,sqlite

COPY --from=planner /app/recipe.json recipe.json

# Install libs needed for building dependencies (like sqlx)
RUN apt-get update && apt-get install -y libsqlite3-dev pkg-config

# Cook the dependencies, which will be cached
RUN cargo chef cook --release --recipe-path recipe.json

## --- Stage 3: Frontend Builder (Node.js Dependencies) ---
# This new stage handles building the Svelte frontend and caches node_modules.
FROM node:20-slim AS frontend-builder
WORKDIR /app
# Copy only the package manifests first
COPY frontend_svelte/package.json frontend_svelte/package-lock.json* ./
# Install dependencies. This layer is cached and only re-runs if manifests change.
RUN npm install
# Copy the rest of the frontend source code
COPY frontend_svelte/ ./
# Build the production version of the frontend
RUN npm run build

# --- Stage 4: App Builder (Final Assembly) ---
# This stage combines the cached dependencies with the application code to build the final binary.
FROM cacher as app-builder
WORKDIR /app

# Copy the cached Rust dependencies from the 'cacher' stage
COPY --from=cacher /app/target /app/target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

# Copy the entire project source code
COPY . .

## --- Frontend Integration ---
# Copy the pre-built frontend from the 'frontend-builder' stage
COPY --from=frontend-builder /app/build ./backend/static/svelte-build
## --------------------------

# Set up the database and run migrations
ENV APP_JWT_SECRET="build-time-secret"
ENV DATABASE_URL="sqlite:backend/database.db"
RUN sqlx database create
RUN sqlx migrate run --source backend/migrations

# Build the final backend binary. This is fast because dependencies are cached.
# Note: No feature flag needed here as we are only building for svelte now.
# If you need to switch, you'd re-introduce the ARG.
RUN cargo build --release --package backend --features svelte-ui

# --- Stage 5: Runtime ---
# This is the final, small image that will run in production.
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy only the necessary artifacts from the 'app-builder' stage
COPY --from=app-builder /app/target/release/backend ./cornerstone-server
COPY --from=app-builder /app/backend/static/svelte-build ./backend/static/svelte-build
COPY --from=app-builder /app/backend/database.db ./backend/database.db
COPY --from=app-builder /app/Config.toml .

EXPOSE 8080
ENV APP_DATABASE__URL=sqlite:/app/backend/database.db
ENV RUST_LOG="info"
CMD ["./cornerstone-server"]