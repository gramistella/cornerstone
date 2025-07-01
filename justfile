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
# This is the fastest way to see if your code compiles.
check:
    @echo "✅ Checking workspace for errors..."
    @cargo check --workspace

# Lint the workspace for style and correctness issues
lint:
    @echo "🔎 Linting workspace with Clippy..."
    @cargo clippy --workspace --all-targets -- -D warnings

# Check all SQL queries against the running database at compile time
db-prepare:
    @echo "🗄️ Preparing SQLx queries..."
    @cargo sqlx prepare --workspace -- --all-targets

# Build a specific package by name.
# USAGE: just build backend | just build frontend_slint | just build common
build package:
    @echo "📦 Building package: '{{package}}'..."
    @cargo build -p {{package}}

# Build all packages in the workspace.
build-all: build-backend build-wasm
    @echo "📦 Building all workspace packages..."

# Convenience alias to build only the frontend.
# Useful for updating Slint macro expansions for rust-analyzer.
build-frontend-slint:
    @just build "frontend-slint"

# Convenience alias to build only the backend.
build-backend:
    @just build "backend"

# Build the frontend for WebAssembly using wasm-pack.
# This creates the necessary .js and .wasm files in the frontend/pkg directory.
build-wasm:
    @echo "🧹 Cleaning wasm-pack output folder (keeping .gitkeep)..."
    @find frontend_slint/static/wasm-pack -type f ! -name '.gitkeep' -delete
    @find frontend_slint/static/wasm-pack -type d -empty -not -path 'frontend_slint/static/wasm-pack' -delete
    @echo "📦 Building Slint's WebAssembly frontend..."
    @cd frontend_slint && wasm-pack build --target web --out-dir static/wasm-pack
    @rm -f frontend_slint/static/wasm-pack/.gitignore

gen-types:
    @echo "TypeScript: Generating type definitions..."
    # Run the `type-exporter` binary from the `common` crate,
    # activating the `ts_export` feature which enables it.
    @cargo run -p common --features ts_export --bin type-exporter
    # Optional: format the generated TS file
    @npx prettier --write frontend_svelte/src/lib/types.ts

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
test-backend: db-migrate
    @just test-package "backend"


# -----------------------------------------------------------------------------
# # Database Commands
# -----------------------------------------------------------------------------

# Run database migrations using sqlx-cli.
# NOTE: This requires `sqlx-cli` to be installed (`cargo install sqlx-cli`).
# The path is hardcoded here to ensure it runs correctly from the root.
db-migrate:
    @echo "🗄️ Running database migrations..."
    @sqlx migrate run --database-url 'sqlite:backend/database.db' --source backend/migrations

# -----------------------------------------------------------------------------
# # Deployment & Execution Commands
# -----------------------------------------------------------------------------

# NEW: Copy the built WASM app to the backend's static directory.
copy-frontend:
    @echo "-  Copying frontend files to backend/static/..."
    @mkdir -p backend/static/wasm
    @rsync -av --exclude='.gitkeep' frontend_slint/static/wasm-pack/ backend/static/wasm/
    @echo "   ...done"

# Build and run the backend server.
run-backend: build-backend
    @echo "🚀 Starting backend server..."
    @cargo run -p backend

# Build both frontend and backend, copy assets, and then run the server.
run-web: build-wasm copy-frontend run-backend

# -----------------------------------------------------------------------------
# # Development Workflow Commands
# -----------------------------------------------------------------------------

# Watch for file changes in relevant crates and automatically rebuild & restart.
# This is great for rapid development.
# NOTE: Requires `cargo-watch` (`cargo install cargo-watch`).
watch:
    @echo "👀 Watching for changes... (Backend will restart on save)"
    @cargo watch -q -c -w backend -w common -w frontend_slint/src -w frontend_slint/ui -x "run-web"

# -----------------------------------------------------------------------------
# # Clean Commands
# -----------------------------------------------------------------------------

# Clean build artifacts by removing the `target` directory.
clean:
    @echo "🧹 Cleaning build artifacts..."
    @rm -rf ./target
    @find ./frontend_slint/static/wasm-pack -type f ! -name '.gitkeep' -delete
    @find ./frontend_slint/static/wasm-pack -type d -empty -not -path './frontend_slint/static/wasm-pack' -delete
    @find ./backend/static/wasm -type f ! -name '.gitkeep' -delete
    @find ./backend/static/wasm -type d -empty -not -path './backend/static/wasm' -delete

# Remove all build artifacts AND the development database.
# DANGER: This is destructive and will delete your local database file.
distclean: clean
    @echo "🔥 Removing database file..."
    @rm -f backend/database.db

# Format all Rust code in the workspace
fmt:
    @echo "💅 Formatting all Rust code..."
    @cargo fmt --all

# DANGER: Deletes and recreates the database, then runs all migrations.
# This will ask for confirmation before proceeding.
[confirm("⚠️  This will DELETE the current database. Are you sure?")]
db-reset:
    @echo "-> Proceeding with database reset..."
    @echo "  - Deleting old database..."
    @rm -f backend/database.db
    @echo "  - Creating a new database file..."
    @touch backend/database.db
    @echo "  - Running migrations to recreate the database and schema..."
    @just db-migrate
    @echo "✨ Database reset complete."