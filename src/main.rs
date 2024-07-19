use axum::{routing::get, Router};
use tower_http::services::ServeFile;

mod stream;
mod watcher;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route_service("/", ServeFile::new("assets/main.html"))
        .route("/watch", get(stream::watch));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
