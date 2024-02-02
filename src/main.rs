use axum::{
    body::Body,
    http::Response,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use hyper::header::CONTENT_TYPE;
use mime::TEXT_CSS;
use mime::TEXT_JAVASCRIPT;

const ADDR: &'static str = "127.0.0.1:8080"; // Sets the address to listen on

#[tokio::main]
async fn main() {
    let app = Router::new() // Creates a new router
        .route("/", get(idx_handler)) // Handles get requests for the index of the app
        .route("/style.css", get(css_handler)) // Handles get requests for the css of the app
        .route("/htmx.min.js", get(htmx_handler))
        .route("/hu", post(hu_handler)); // Handles get requests for the htmx library

    println!("Listening on: {}\n", ADDR);
    let listener = tokio::net::TcpListener::bind(ADDR).await.unwrap(); // Binds the listener to the address
    axum::serve(listener, app).await.unwrap(); // Serves the app
}

async fn idx_handler() -> Html<&'static str> {
    Html(include_str!("html/index.html")) // Serves the contents of index.html
}

async fn css_handler() -> impl IntoResponse {
    println!(" -> SERVE: style.css");
    let body = include_str!("html/style.css");
    let body = Body::from(body);
    Response::builder()
        .header(CONTENT_TYPE, TEXT_CSS.to_string())
        .body(body)
        .unwrap()
}

async fn htmx_handler() -> impl IntoResponse {
    println!(" -> SERVE: htmx.min.js");
    let body = include_str!("html/htmx.min.js");
    let body = Body::from(body);
    Response::builder()
        .header(CONTENT_TYPE, TEXT_JAVASCRIPT.to_string())
        .body(body)
        .unwrap()
}


async fn hu_handler() {
    println!("Home point up");
}
