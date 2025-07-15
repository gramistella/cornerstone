# Use BASH for more advanced shell features
set shell := ["bash", "-euc"]

# Load environment variables from .env files
set dotenv-load := true


# Default command to run when you just type `just`
default: check

# -----------------------------------------------------------------------------
# # Build & Check Commands
# -----------------------------------------------------------------------------

# Check the entire workspace for errors without a full build.
check:
    @echo "✅ Checking workspace for errors..."
    @cargo check --workspace

# Generic lint command (for local use, may use default features)
lint:
    @echo "🔎 Linting workspace with Clippy..."
    @cargo clippy --workspace --all-targets -- -D warnings

# Lint specifically for SQLite
lint-sqlite:
    @echo "🔎 Linting workspace for SQLite..."
    @cargo clippy --workspace --all-targets --no-default-features --features "db-sqlite,svelte-ui" -- -D warnings

# Lint specifically for PostgreSQL
lint-postgres:
    @echo "🔎 Linting workspace for PostgreSQL..."
    @cargo clippy --workspace --all-targets --no-default-features --features "db-postgres,svelte-ui" -- -D warnings

# Check all SQL queries against the running database at compile time
db-prepare:
    @echo "🗄️ Preparing SQLx queries..."
    @cargo sqlx prepare --workspace -- --all-targets

# Build a specific package by name.
# USAGE: just build backend | just build frontend_slint | just build common
build package:
    @echo "📦 Building package: '{{package}}'..."
    @cargo build -p {{package}}

# Convenience alias to build only the backend.
build-backend:
    @just build "backend"

# Build the SvelteKit frontend for web.
build-svelte:
    @echo "📦 Building SvelteKit frontend..."
    @cd frontend_svelte && npm install && npm run build
    @echo "  -> SvelteKit build complete."

# Build the Slint frontend for WebAssembly using wasm-pack.
build-slint:
    @echo "🧹 Cleaning wasm-pack output folder (keeping .gitkeep)..."
    @find frontend_slint/static/wasm-pack -type f ! -name '.gitkeep' -delete
    @find frontend_slint/static/wasm-pack -type d -empty -not -path 'frontend_slint/static/wasm-pack' -delete
    @echo "📦 Building Slint's WebAssembly frontend..."
    @cd frontend_slint && wasm-pack build --target web --out-dir static/wasm-pack
    @rm -f frontend_slint/static/wasm-pack/.gitignore

# Build all packages in the workspace (both Rust and frontend projects)
build-all: build-backend build-svelte build-slint
    @echo "📦 Building all workspace packages..."

# Generate TypeScript types from Rust structs.
gen-types:
    @echo "TypeScript: Generating type definitions..."
    # Run the `type-exporter` binary from the `common` crate,
    # activating the `ts_export` feature which enables it.
    @cargo run -p common --features ts_export --bin type-exporter

# -----------------------------------------------------------------------------
# # Test Commands
# -----------------------------------------------------------------------------

# Run all tests in the workspace.
test:
    @echo "🧪 Running all workspace tests..."
    @cargo test --workspace

# Run tests for a specific package.
# USAGE: just test-package backend
test-package package:
    @echo "🧪 Running tests for package: '{{package}}'..."
    @cargo test -p {{package}}

# Convenience alias to run backend tests.
# Ensures the database is migrated before running tests.
#test-backend: db-migrate
#    @just test-package "backend"

# -----------------------------------------------------------------------------
# # Database Commands
# -----------------------------------------------------------------------------

# Run database migrations for SQLite
db-migrate-sqlite:
    @echo "🗄️ Running SQLite database migrations..."
    @sqlx migrate run --source backend/migrations/sqlite --database-url "$DATABASE_URL_SQLITE"

# Run database migrations for PostgreSQL
db-migrate-postgres:
    @echo "🗄️ Running PostgreSQL database migrations..."
    @sqlx migrate run --source backend/migrations/postgres --database-url "$DATABASE_URL"

# DANGER: Drops and recreates the database, then runs all migrations.
[confirm("⚠️ This will DELETE the current PostgreSQL database. Are you sure?")]
db-reset-postgres:
    @echo "-> Dropping and recreating PostgreSQL database..."
    @sqlx database drop --database-url "$DATABASE_URL"
    @sqlx database create --database-url "$DATABASE_URL"
    @just db-migrate-postgres

[confirm("⚠️ This will DELETE the current SQLite database file. Are you sure?")]
db-reset-sqlite:
    @echo "-> Deleting and recreating SQLite database file..."
    @rm -f backend/database.db
    @touch backend/database.db
    @just db-migrate-sqlite

# Run backend tests against SQLite
test-backend-sqlite: db-migrate-sqlite
    @echo "🧪 Running backend tests against SQLite..."
    # Add --no-default-features to be explicit
    @cargo test -p backend --no-default-features --features "db-sqlite,svelte-ui"

# Run backend tests against PostgreSQL
test-backend-postgres: db-migrate-postgres
    @echo "🧪 Running backend tests against PostgreSQL..."
    # Add --no-default-features to prevent the default 'db-sqlite' from being included
    @cargo test -p backend --no-default-features --features "db-postgres,svelte-ui"

# -----------------------------------------------------------------------------
# # Deployment & Execution Commands
# -----------------------------------------------------------------------------

# Copy the built frontend to the backend's static directory.
copy-frontend frontend:
    @echo "- Copying frontend files to backend/static/..."
    @if [ "{{frontend}}" = "svelte" ]; then \
        mkdir -p backend/static/svelte-build; \
        find backend/static/svelte-build -mindepth 1 ! -name '.gitkeep' -exec rm -rf {} +; \
        echo "  -> Copying SvelteKit build..."; \
        cp -r frontend_svelte/build/* backend/static/svelte-build/; \
    elif [ "{{frontend}}" = "slint" ]; then \
        mkdir -p backend/static/slint-build/wasm; \
        find backend/static/slint-build -mindepth 1 \
            ! -name '.gitkeep' \
            ! -name 'index.html' \
            ! -path 'wasm/.gitkeep' \
            -exec rm -rf {} +; \
        echo "  -> Copying Slint WASM build..."; \
        cp -r frontend_slint/static/wasm-pack/* backend/static/slint-build/wasm/; \
    else \
        echo "  -> Unknown frontend '{{frontend}}'. Aborting."; \
        exit 1; \
    fi
    @echo "  ...done"


# Build and run the specified frontend with the backend, or run in live-dev mode.
# USAGE: just run-web svelte | just run-web slint | just run-web svelte-live
run-web frontend="svelte":
    @if [ "{{frontend}}" = "svelte-live" ]; then \
        echo "🚀 Starting backend and SvelteKit dev server in parallel..."; \
        echo "  -> Backend API will be at http://localhost:8080"; \
        echo "  -> Svelte dev server will be at http://localhost:5173"; \
        cd frontend_svelte && npx concurrently --kill-others --names "svelte,backend" "npm run dev" "cd .. && just run-backend"; \
    else \
        echo "📦 Building and running with static frontend: {{frontend}}"; \
        just build-{{frontend}}; \
        just copy-frontend {{frontend}}; \
        just run-backend; \
    fi


# Build and run just the backend server.
run-backend: build-backend
    @echo "🚀 Starting backend server..."
    @cargo run -p backend


# -----------------------------------------------------------------------------
# # Development Workflow Commands
# -----------------------------------------------------------------------------


# Watch for file changes in relevant crates and automatically rebuild & restart.
# This is great for rapid development.
# NOTE: Requires `cargo-watch` (`cargo install cargo-watch`).
watch-slint:
    @echo "👀 Watching for changes... (Backend + Slint)"
    @cargo watch -q -c \
      -w backend \
      -w common \
      -w frontend_slint/src \
      -w frontend_slint/ui \
      -x "run-web slint"

watch-svelte:
    @echo "👀 Watching for changes... (Backend + SvelteKit)"
    @echo "Note: The SvelteKit dev server runs separately. Run 'npm run dev' in the frontend_svelte directory."
    @echo "This command will only watch and restart the backend."
    @cargo watch -q -c -w backend -w common -x "run-backend"

# -----------------------------------------------------------------------------
# # Clean Commands
# -----------------------------------------------------------------------------

# Clean build artifacts by removing the `target` directory.
clean:
    @echo "🧹 Cleaning build artifacts..."
    @rm -rf ./target
    @rm -rf ./frontend_svelte/build
    @find ./frontend_slint/static/wasm-pack -type f ! -name '.gitkeep' -delete
    @find ./frontend_slint/static/wasm-pack -type d -empty -not -path './frontend_slint/static/wasm-pack' -delete

    @echo "🧹 Cleaning SvelteKit build artifacts..."
    @find ./backend/static/svelte-build -mindepth 1 ! -name '.gitkeep' -exec rm -rf {} +

    @echo "🧹 Cleaning Slint build artifacts..."
    @find ./backend/static/slint-build -mindepth 1 \
        \( -path './backend/static/slint-build/wasm' -prune \) -o \
        \( ! -name '.gitkeep' ! -name 'index.html' -exec rm -rf {} + \)

    @find ./backend/static/slint-build/wasm -mindepth 1 ! -name '.gitkeep' -exec rm -rf {} +

# Remove all build artifacts AND the development database.
# DANGER: This is destructive and will delete your local database file.
distclean: clean
    @echo "🔥 Removing database file..."
    @rm -f backend/database.db

# Format all Rust code in the workspace
fmt:
    @echo "💅 Formatting all Rust code..."
    @cargo fmt --all
