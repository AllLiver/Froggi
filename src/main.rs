use axum::{
    body::Body,
    http::Response,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post, put},
    Form, Router,
};

use lazy_static::lazy_static;
use std::{path::Path, sync::Mutex};

use hyper::header::CONTENT_TYPE;
use mime::IMAGE_PNG;
use mime::TEXT_CSS;
use mime::TEXT_JAVASCRIPT;

use serde::Deserialize;

const ADDR: &'static str = "127.0.0.1:8080"; // Sets the address to listen on
const CONFIG_FILE: &'static str = "config.cfg"; // Sets the name of the config file

lazy_static! {
    static ref HOME_NAME: Mutex<String> = Mutex::new(String::from("home_name"));
    static ref AWAY_NAME: Mutex<String> = Mutex::new(String::from("home_name"));
    static ref HOME_POINTS: Mutex<i32> = Mutex::new(0);
    static ref AWAY_POINTS: Mutex<i32> = Mutex::new(0);
    static ref TIME_MINS: Mutex<i32> = Mutex::new(8);
    static ref TIME_SECS: Mutex<i32> = Mutex::new(0);
    static ref TIME_STARTED: Mutex<bool> = Mutex::new(false);
    static ref CHROMAKEY: Mutex<(u8, u8, u8)> = Mutex::new((0, 0, 0));
    static ref QUARTER: Mutex<i32> = Mutex::new(1);
    static ref SHOW_QUARTER: Mutex<bool> = Mutex::new(true);
}

#[tokio::main]
async fn main() {
    let app = Router::new() // Creates a new router
        .route("/", get(idx_handler)) // Handles get requests for the index of the app
        .route("/chromakey", get(chroma_handler)) // Handles get requests for the chromakey page
        .route("/style.css", get(css_handler)) // Handles get requests for the css of the app
        .route("/htmx.min.js", get(htmx_handler)) // Handles get requests for the htmx library
        .route("/hu", post(hu_handler))
        .route("/hd", post(hd_handler))
        .route("/hu2", post(hu2_handler))
        .route("/hu3", post(hu3_handler))
        .route("/hp", put(hp_handler))
        .route("/home_png", get(home_img_handler))
        .route("/au", post(au_handler))
        .route("/ad", post(ad_handler))
        .route("/au2", post(au2_handler))
        .route("/au3", post(au3_handler))
        .route("/ap", put(ap_handler))
        .route("/away_png", get(away_img_handler))
        .route("/tstart", post(tstart_handler))
        .route("/tstop", post(tstop_handler))
        .route("/time", put(time_handler))
        .route("/time_secs", put(secs_handler))
        .route("/time_mins", put(mins_handler))
        .route("/mins_up", post(mins_up_handler))
        .route("/mins_down", post(mins_down_handler))
        .route("/secs_up", post(secs_up_handler))
        .route("/secs_down", post(secs_down_handler))
        .route("/", post(tname_handler))
        .route("/hdisp", put(hdisp_handler))
        .route("/adisp", put(adisp_handler))
        .route("/chromargb", put(chromargb_handler))
        .route("/hname_score", put(hname_scoreboard_handler))
        .route("/aname_score", put(aname_scoreboard_handler))
        .route("/quarter", put(quarter_handler))
        .route("/q1", post(quarter1_change))
        .route("/q2", post(quarter2_change))
        .route("/q3", post(quarter3_change))
        .route("/q4", post(quarter4_change));

    tokio::spawn(clock_ticker());
    tokio::spawn(read_or_create_config());

    println!("Listening on: {}\n", ADDR);
    let listener = tokio::net::TcpListener::bind(ADDR).await.unwrap(); // Binds the listener to the address
    axum::serve(listener, app).await.unwrap(); // Serves the app
    println!("Server stopped");
}

// region: --- Config fn's

async fn read_or_create_config() {
    let config = match tokio::fs::read_to_string(CONFIG_FILE).await {
        Ok(cfg) => cfg,
        Err(_) => {
            println!(" -> CREATE: config file");
            tokio::fs::write(CONFIG_FILE, "# FOSSO config file\nchromakey=0, 177, 64")
                .await
                .unwrap();
            tokio::fs::read_to_string(CONFIG_FILE).await.unwrap()
        }
    };

    let lines: Vec<&str> = config.split('\n').filter(|x| !x.starts_with("#")).collect();
    println!(" -> CONFIG: {:?}", lines);

    for i in lines {
        let parts: Vec<&str> = i.split('=').collect();
        match parts[0] {
            "chromakey" => {
                let rgb: Vec<&str> = parts[1].split(',').collect();
                let r: u8 = rgb[0].trim().parse().unwrap();
                let g: u8 = rgb[1].trim().parse().unwrap();
                let b: u8 = rgb[2].trim().parse().unwrap();
                let mut chromakey = CHROMAKEY.lock().unwrap();
                *chromakey = (r, g, b);
            }
            _ => println!(" -> CONFIG: unknown config: {}", parts[0]),
        }
    }
}

// endregion: --- Config fn's
// region: --- Page handlers

async fn idx_handler() -> Html<&'static str> {
    println!(" -> SERVE: index.html");
    Html(include_str!("html/index.html")) // Serves the contents of index.html
}

async fn chroma_handler() -> Html<&'static str> {
    println!(" -> SERVE: chromakey.html");
    Html(include_str!("html/scoreboard/chromakey.html"))
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

async fn hname_scoreboard_handler() -> Html<String> {
    let home_name = HOME_NAME.lock().unwrap();
    Html(format!("{}", home_name))
}

async fn aname_scoreboard_handler() -> Html<String> {
    let away_name = AWAY_NAME.lock().unwrap();
    Html(format!("{}", away_name))
}

async fn home_img_handler() -> impl IntoResponse {
    let home_image = tokio::fs::read(Path::new("home.png")).await.unwrap();
    let body = Body::from(home_image);
    Response::builder()
        .header(CONTENT_TYPE, IMAGE_PNG.to_string())
        .body(body)
        .unwrap()
}

async fn away_img_handler() -> impl IntoResponse {
    let away_image = tokio::fs::read(Path::new("away.png")).await.unwrap();
    let body = Body::from(away_image);
    Response::builder()
        .header(CONTENT_TYPE, IMAGE_PNG.to_string())
        .body(body)
        .unwrap()
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
    Html(format!("{}", *home_points))
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
    Html(format!("{}", *away_points))
}

// endregion: --- Away Handlers
// region: --- Clock handlers

async fn time_handler() -> Html<String> {
    let time_mins = TIME_MINS.lock().unwrap();
    let time_secs = TIME_SECS.lock().unwrap();
    Html(format!("{}:{:02?}", *time_mins, *time_secs))
}

async fn mins_handler() -> Html<String> {
    let time_mins = TIME_MINS.lock().unwrap();
    Html(format!("{}", *time_mins))
}

async fn secs_handler() -> Html<String> {
    let time_secs = TIME_SECS.lock().unwrap();
    Html(format!("{:02?}", *time_secs))
}

async fn mins_up_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    *time_mins += 1;
}

async fn mins_down_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    if *time_mins > 0 {
        *time_mins -= 1;
    }
}

async fn secs_up_handler() {
    let mut time_secs = TIME_SECS.lock().unwrap();
    if *time_secs < 59 {
        *time_secs += 1;
    }
}

async fn secs_down_handler() {
    let mut time_secs = TIME_SECS.lock().unwrap();
    if *time_secs > 0 {
        *time_secs -= 1;
    }
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
// region: --- Quarter handlers

async fn quarter_handler() -> Html<String> {
    let quarter = QUARTER.lock().unwrap();
    Html(format!("{}", *quarter))
}

async fn quarter1_change() {
    let mut quarter = QUARTER.lock().unwrap();
    *quarter = 1;
}

async fn quarter2_change() {
    let mut quarter = QUARTER.lock().unwrap();
    *quarter = 2;
}

async fn quarter3_change() {
    let mut quarter = QUARTER.lock().unwrap();
    *quarter = 3;
}

async fn quarter4_change() {
    let mut quarter = QUARTER.lock().unwrap();
    *quarter = 4;
}

// endregion: --- Quarter handlers
// region: --- Misc handelers

async fn chromargb_handler() -> Html<String> {
    let chromakey = CHROMAKEY.lock().unwrap();
    Html(format!(
        "<style>body {{ background-color: rgb({}, {}, {}); }}</style>",
        chromakey.0, chromakey.1, chromakey.2
    ))
}

// endregion: --- Misc handelers
