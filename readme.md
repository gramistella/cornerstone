For the frontend as standalone:
1. Run `cargo build` in the frontend directory to build the frontend
2. Run `cargo run` in the frontend directory to start the frontend as standalone

For the backend:
1. Run `cargo run` in the backend directory to start the backend
2. Remove the build.rs file in the backend directory to prevent the wasm files from being copied to the backend/static/wasm directory

For the frontend as a web app:
1. Run `wasm-pack build --target web --out-dir static/wasm-pack` in the frontend directory to build the frontend as wasm
2. Run `cargo run` in the backend directory to start the backend
3. The frontend will be available at http://localhost:8080
cd ..