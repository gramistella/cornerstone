# Build the WASM frontend
build-wasm:
    cd frontend && wasm-pack build --target web --out-dir static/wasm-pack

# Build WASM, copy the files, and then run the server
run-web: build-wasm
    @echo "--- Copying WASM files to backend ---"
    @mkdir -p backend/static/wasm
    @cp -r frontend/static/wasm-pack/* backend/static/wasm/
    cd backend && cargo run

# Build and run the backend server (without building/copying wasm)
run-server:
    cd backend && cargo run

# Run the native desktop app
run-desktop:
    cd frontend && cargo run

# Run tests for all crates in the workspace
test:
    cargo test --workspace

# Clean all build artifacts and WASM output
clean:
    cargo clean
    @echo "--- Removing WASM output ---"
    @rm -rf frontend/static/wasm-pack
    @rm -rf backend/static/wasm
    @echo "--- Clean complete ---"
