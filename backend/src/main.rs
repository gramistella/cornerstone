mod web_server;

use tokio::join;

#[tokio::main]
async fn main() {
    // Start serving the frontend WebAssembly
    let server = web_server::run_server();

    // Other tasks

    // Run tasks concurrently if needed
    join!(server);
}
