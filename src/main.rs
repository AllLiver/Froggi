use axum::{
    body::Body,
    http::Response,
    response::{Html, IntoResponse},
    routing::{get, post, put},
    Router
};

use lazy_static::lazy_static;
use std::sync::Mutex;

use hyper::header::CONTENT_TYPE;
use mime::TEXT_CSS;
use mime::TEXT_JAVASCRIPT;

const ADDR: &'static str = "127.0.0.1:8080"; // Sets the address to listen on

lazy_static! {
    static ref COUNTER: Mutex<i32> = Mutex::new(0);
}

#[tokio::main]
async fn main() {
    let app = Router::new() // Creates a new router
        .route("/", get(idx_handler)) // Handles get requests for the index of the app
        .route("/style.css", get(css_handler)) // Handles get requests for the css of the app
        .route("/htmx.min.js", get(htmx_handler))
        .route("/hu", post(hu_handler)) // Handles get requests for the htmx library
        .route("/hp", put(hp_handler));

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
    let mut counter = COUNTER.lock().unwrap();
    *counter += 1;
    println!("Home point up, points: {}", *counter);
}

async fn hp_handler() -> Html<String> {
    let counter = COUNTER.lock().unwrap();
    Html(format!("Points: {}", *counter))
}