use axum::{
    routing::get_service,
    Router,
};
use tower_http::services::ServeDir;  
use std::net::SocketAddr;

pub async fn run_server() {
    let app = Router::new().nest_service("/", get_service(ServeDir::new("static")).handle_error(|_| async {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong...")
    }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap();

    println!("Serving frontend at http://{}", addr);
    
    axum::serve(listener, app).await.unwrap();
       
}
