use axum::{
    body::Body,
    http::Response,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post, put},
    Form, Router,
};

use lazy_static::lazy_static;
use std::sync::Mutex;

use hyper::header::CONTENT_TYPE;
use mime::TEXT_CSS;
use mime::TEXT_JAVASCRIPT;

use serde::Deserialize;

const ADDR: &'static str = "127.0.0.1:8080"; // Sets the address to listen on

lazy_static! {
    static ref HOME_NAME: Mutex<String> = Mutex::new(String::from("home_name"));
    static ref AWAY_NAME: Mutex<String> = Mutex::new(String::from("home_name"));
    static ref HOME_POINTS: Mutex<i32> = Mutex::new(0);
    static ref AWAY_POINTS: Mutex<i32> = Mutex::new(0);
    static ref TIME_MINS: Mutex<i32> = Mutex::new(8);
    static ref TIME_SECS: Mutex<i32> = Mutex::new(20);
    static ref TIME_STARTED: Mutex<bool> = Mutex::new(false);
}

#[tokio::main]
async fn main() {
    let app = Router::new() // Creates a new router
        .route("/", get(idx_handler)) // Handles get requests for the index of the app
        .route("/style.css", get(css_handler)) // Handles get requests for the css of the app
        .route("/htmx.min.js", get(htmx_handler)) // Handles get requests for the htmx library
        .route("/hu", post(hu_handler))
        .route("/hd", post(hd_handler))
        .route("/hu2", post(hu2_handler))
        .route("/hu3", post(hu3_handler))
        .route("/hp", put(hp_handler))
        .route("/au", post(au_handler))
        .route("/ad", post(ad_handler))
        .route("/au2", post(au2_handler))
        .route("/au3", post(au3_handler))
        .route("/ap", put(ap_handler))
        .route("/tstart", post(tstart_handler))
        .route("/tstop", post(tstop_handler))
        .route("/time", put(time_handler))
        .route("/", post(tname_handler))
        .route("/hdisp", put(hdisp_handler))
        .route("/adisp", put(adisp_handler));

    tokio::spawn(clock_ticker());

    println!("Listening on: {}\n", ADDR);
    let listener = tokio::net::TcpListener::bind(ADDR).await.unwrap(); // Binds the listener to the address
    axum::serve(listener, app).await.unwrap(); // Serves the app
    println!("Server stopped");
}



// region: --- Page handlers

async fn idx_handler() -> Html<&'static str> {
    println!(" -> SERVE: index.html");
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

// endregion: --- Page handlers

// region: --- Team names

#[derive(Deserialize)]
struct UpdNames {
    home: String,
    away: String,
}

async fn tname_handler(Form(names): Form<UpdNames>) -> Redirect {
    println!(" -> TEAMS: update names: {} - {}", names.home, names.away);
    let mut home_name = HOME_NAME.lock().unwrap();
    let mut away_name = AWAY_NAME.lock().unwrap();
    *home_name = names.home;
    *away_name = names.away;

    Redirect::to("/")
}

async fn hdisp_handler() -> Html<String> {
    let home_name = HOME_NAME.lock().unwrap();
    Html(format!("<h2>Home: {}</h2>", home_name))
}

async fn adisp_handler() -> Html<String> {
    let away_name = AWAY_NAME.lock().unwrap();
    Html(format!("<h2>Away: {}</h2>", away_name))
}

// endregion: --- Team names

// region: --- Home handlers

async fn hu_handler() {
    // Increments home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    *home_points += 1;
}

async fn hd_handler() {
    // Decrements home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    if *home_points > 0 {
        *home_points -= 1;
    }
}

async fn hu2_handler() {
    // Adds 2 home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    *home_points += 2;
}

async fn hu3_handler() {
    // Adds 3 home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    *home_points += 3;
}

async fn hp_handler() -> Html<String> {
    // Displays home points
    let home_points = HOME_POINTS.lock().unwrap();
    Html(format!("Points: {}", *home_points))
}

// endregion: --- Home handlers

// region: --- Away handlers

async fn au_handler() {
    // Increments home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    *away_points += 1;
}

async fn ad_handler() {
    // Decrements home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    if *away_points > 0 {
        *away_points -= 1;
    }
}

async fn au2_handler() {
    // Adds 2 home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    *away_points += 2;
}

async fn au3_handler() {
    // Adds 3 home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    *away_points += 3;
}

async fn ap_handler() -> Html<String> {
    // Displays home points
    let away_points = AWAY_POINTS.lock().unwrap();
    Html(format!("Points: {}", *away_points))
}

// endregion: --- Away Handlers

// region: --- Clock handlers

async fn time_handler() -> Html<String> {
    let time_mins = TIME_MINS.lock().unwrap();
    let time_secs = TIME_SECS.lock().unwrap();
    Html(format!("{}:{:02?}", *time_mins, *time_secs))
}

async fn clock_ticker() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut time_started = TIME_STARTED.lock().unwrap();
        if *time_started {
            let mut time_mins = TIME_MINS.lock().unwrap();
            let mut time_secs = TIME_SECS.lock().unwrap();
            if *time_secs == 0 {
                if *time_mins == 0 {
                    *time_started = false;
                } else {
                    *time_mins -= 1;
                    *time_secs = 59;
                }
            } else {
                *time_secs -= 1;
            }
        }
    }
}

async fn tstart_handler() {
    println!(" -> TIMER: start");
    let mut time_started = TIME_STARTED.lock().unwrap();
    *time_started = true;
}

async fn tstop_handler() {
    println!(" -> TIMER: stop");
    let mut time_started = TIME_STARTED.lock().unwrap();
    *time_started = false;
}

// endregion: --- Clock handlers
