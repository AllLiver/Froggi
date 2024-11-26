// Froggi routing (basic)

use axum::{
    body::Body,
    http::{header::LOCATION, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};
use reqwest::header::CONTENT_TYPE;
use tokio::fs::File;

pub async fn index_handler() -> impl IntoResponse {
    Html::from(include_str!("../html/index.html"))
}

pub async fn teaminfo_handler() -> impl IntoResponse {
    Html::from(include_str!("../html/teaminfo.html"))
}

pub async fn overlay_handler() -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("../html/overlay.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login/create"))
            .body(String::new())
            .unwrap();
    }
}

pub async fn settings_handler() -> impl IntoResponse {
    Html::from(include_str!("../html/settings.html"))
}

pub async fn logs_page_handler() -> impl IntoResponse {
    Html::from(include_str!("../html/logs.html"))
}

pub async fn css_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(String::from(include_str!("../html/styles.css")))
        .unwrap()
}

pub async fn not_found_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(String::from(include_str!("../html/status_codes/404.html")))
        .unwrap()
}

pub async fn htmx_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("../html/js/htmx.js")))
        .unwrap()
}

pub async fn ws_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("../html/js/ws.js")))
        .unwrap()
}

pub async fn app_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("../html/js/app.js")))
        .unwrap()
}

pub async fn favicon_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static("image/x-icon"))
        .body(Body::from(
            include_bytes!("../html/img/favicon.png").to_vec(),
        ))
        .unwrap()
}
