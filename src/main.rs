// Brings the axum backend into scope
use axum::{
    body::Body,
    extract::Multipart,
    http::Response,
    response::{Html, IntoResponse, Redirect},
    routing::{get, head, post, put},
    Form, Router,
};

// Brings libraries needed for global variables into scope
use lazy_static::lazy_static;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

use std::path::Path;

// Brings libraries needed for the server headers into scope
use hyper::{header::CONTENT_TYPE, StatusCode};
use mime::IMAGE_PNG;
use mime::TEXT_CSS;
use mime::TEXT_JAVASCRIPT;

// Brings libraries needed for JSON parsing into scope
use serde::{Deserialize, Serialize};

// Brings standard libraries needed for many things into scope
use std::io::{self, BufRead};

// Used for sponsor roll
use base64::prelude::*;

use rand::{thread_rng, Rng};

const CONFIG_FILE: &'static str = "config.cfg"; // Sets the name of the config file

// Declares and intializes all the global variables used everywhere in the app
lazy_static! {
    static ref HOME_NAME: Mutex<String> = Mutex::new(String::from("team_name"));
    static ref AWAY_NAME: Mutex<String> = Mutex::new(String::from("team_name"));
    static ref HOME_POINTS: Mutex<i32> = Mutex::new(0);
    static ref AWAY_POINTS: Mutex<i32> = Mutex::new(0);
    static ref TIME_MINS: Mutex<i32> = Mutex::new(0);
    static ref TIME_SECS: Mutex<i32> = Mutex::new(0);
    static ref TIME_STARTED: Mutex<bool> = Mutex::new(false);
    static ref CHROMAKEY: Mutex<(u8, u8, u8)> = Mutex::new((0, 0, 0));
    static ref QUARTER: Mutex<u8> = Mutex::new(1);
    static ref SHOW_QUARTER: Mutex<bool> = Mutex::new(false);
    static ref ADDR: Mutex<String> = Mutex::new(String::from(""));
    static ref SHOW_SPONSOR: Mutex<bool> = Mutex::new(false);
    static ref LAST_SPONSOR: AtomicUsize = AtomicUsize::from(0);
    static ref SHOW_COUNTDOWN: Mutex<bool> = Mutex::new(false);
    static ref COUNTDOWN_STARTED: Mutex<bool> = Mutex::new(false);
    static ref COUNTDOWN_MINS: Mutex<i32> = Mutex::new(0);
    static ref COUNTDOWN_SECS: Mutex<i32> = Mutex::new(0);
    static ref COUNTDOWN_TITLE: Mutex<String> = Mutex::new(String::from("countdown_title"));
    static ref SPONSOR_IMG_TAGS: Mutex<Vec<Html<String>>> = Mutex::new(Vec::new());
    static ref HOME_IMG_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    static ref AWAY_IMG_DATA: Mutex<Vec<u8>> = Mutex::new(Vec::new());
}

#[tokio::main]
async fn main() {
    std::fs::create_dir_all("./sponsors").unwrap();
    std::fs::create_dir_all("./teams").unwrap();

    *SPONSOR_IMG_TAGS.lock().unwrap() = tokio::spawn(load_sponsors()).await.unwrap();

    tokio::spawn(sponsor_roll_ticker());

    // region: --- Routing

    let app = Router::new() // Creates a new router
        // Routes for the html files, css, and lib files
        .route("/", get(idx_handler)) // Handles get requests for the index of the app
        .route("/overlay", get(chroma_handler)) // Handles get requests for the overlay page
        .route("/teaminfo", get(upload_page_handler)) // Handles get requests for the upload page
        .route("/countdown", get(countdown_handler))
        .route("/style.css", get(css_handler)) // Handles get requests for the css of the app
        .route("/htmx.min.js", get(htmx_handler)) // Handles get requests for the htmx library
        .route("/app.js", get(app_js_handler))
        .route("/favicon_png", get(favicon_handler))
        // Routes to update the home team's info
        .route("/hu", post(hu_handler))
        .route("/hd", post(hd_handler))
        .route("/hu2", post(hu2_handler))
        .route("/hu3", post(hu3_handler))
        .route("/hp", put(hp_handler))
        .route("/home_png", get(home_img_handler))
        // Routes to update the away team's info
        .route("/au", post(au_handler))
        .route("/ad", post(ad_handler))
        .route("/au2", post(au2_handler))
        .route("/au3", post(au3_handler))
        .route("/ap", put(ap_handler))
        .route("/away_png", get(away_img_handler))
        // Routes to update the clock
        .route("/qt8", post(quick_time8_handler))
        .route("/qt5", post(quick_time5_handler))
        .route("/qt3", post(quick_time3_handler))
        .route("/qt1", post(quick_time1_handler))
        .route("/tstart", post(tstart_handler))
        .route("/tstop", post(tstop_handler))
        .route("/time", put(time_handler))
        .route("/time_dashboard", put(dashboard_time_display_handler))
        .route("/mins_up", post(mins_up_handler))
        .route("/mins_down", post(mins_down_handler))
        .route("/secs_up", post(secs_up_handler))
        .route("/secs_down", post(secs_down_handler))
        // Route to update the team name with a POST form
        .route("/", post(tname_handler))
        // Routes to display the team names
        .route("/hdisp", put(hdisp_handler))
        .route("/adisp", put(adisp_handler))
        // Routes for the scoreboard's info and configuration
        .route("/chromargb", put(chromargb_handler))
        .route("/score", put(score_handler))
        .route("/time_and_quarter", put(time_and_quarter_handler))
        .route("/hname_score", put(hname_scoreboard_handler))
        .route("/aname_score", put(aname_scoreboard_handler))
        .route("/quarter", put(quarter_handler))
        .route("/show_quarter", post(quarter_show_handler))
        // Routes to change quarter info
        .route("/change_quarter/:q", post(quarter_change_handler))
        .route("/show_quarter_css", put(show_quarter_css_handler))
        // Routes for team management
        .route("/add_team", post(add_team_handler))
        .route("/load_team/:id", post(load_team_handler))
        .route("/team_selectors", put(team_selectors_handler))
        // Routes for the sponsor roll
        .route("/sponsor_roll", put(sponsor_roll_handler))
        .route("/show_sponsor_roll", post(show_sponsor_roll_handler))
        .route("/sponsor_roll_css", put(sponsor_roll_css_handler))
        // Routes for the countdown
        .route("/show_countdown", post(show_countdown_handler))
        .route("/countdown_css", put(countdown_css_handler))
        .route("/countdown_display", put(countdown_display_handler))
        .route("/qtc20", post(qtc20_handler))
        .route("/qtc15", post(qtc15_handler))
        .route("/qtc10", post(qtc10_handler))
        .route("/qtc5", post(qtc5_handler))
        .route("/countdown_mins_up", post(countdown_mins_up_handler))
        .route("/countdown_mins_down", post(countdown_mins_down_handler))
        .route("/countdown_secs_up", post(countdown_secs_up_handler))
        .route("/countdown_secs_down", post(countdown_secs_down_handler))
        .route("/start_countdown", post(start_countdown_handler))
        .route("/stop_countdown", post(stop_countdown_handler))
        .route("/update_countdown_title", post(countdown_title_handler))
        .route(
            "/countdown_dashboard",
            put(dashboard_countdown_display_handler),
        )
        // Routes to reset the scoreboard
        .route("/reset_scoreboard", post(reset_scoreboard_handler))
        // Routes for the favicon
        .route("/favicon.ico", get(favicon_handler))
        // Routes head requests for calculating latency
        .route("/ping", head(|| async { StatusCode::OK }))
        // Route the 404 page
        .fallback_service(get(|| async {
            println!(" -> 404: not found");
            (StatusCode::NOT_FOUND, Html("<h1>404 - Not Found</h1>"))
        }));

    // endregion: --- Routing

    // Starts the clock tickers
    tokio::spawn(clock_ticker());
    tokio::spawn(countdown_ticker());
    // Opens the config (or creates it if it doesnt exist) file and load configurations
    tokio::spawn(read_or_create_config()).await.unwrap();

    // Gets address from the ADDR mutex
    let listen_addr = ADDR.lock().unwrap();
    let listen_addr: String = listen_addr.clone();

    // Bind the server to the address
    println!(
        "Listening on: {}\nType \"stop\" to do shut down the server gracefully\n",
        listen_addr
    );
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap(); // Binds the listener to the address

    // Creates a oneshot channel to be able to shut down the server gracefully
    let (tx, rx) = tokio::sync::oneshot::channel();

    // Spawns a task to listen for the "stop" command which shuts down the server
    tokio::spawn(async move {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if line.unwrap() == "stop" {
                let _ = tx.send(());
                return;
            }
        }
    });

    // Start the server
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let _ = rx.await;
        println!(" -> SERVER: shutting down");
    });

    // Prints an error if an error occurs whie starting the server
    if let Err(err) = server.await {
        eprintln!(" -> ERROR: {}", err);
    }
    println!(" -> SERVER: gracefully shut down");
}

// region: --- Config fn's

// Function that creates and loads configurations from the config file
async fn read_or_create_config() {
    // Opens or creates the config file if it doesnt exist
    let config = match tokio::fs::read_to_string(CONFIG_FILE).await {
        Ok(cfg) => cfg,
        Err(_) => {
            println!(" -> CREATE: config file");
            tokio::fs::write(
                CONFIG_FILE,
                "# FOSSO config file\nchromakey=0, 177, 64\nlisten_addr=0.0.0.0:8080",
            )
            .await
            .unwrap();
            tokio::fs::read_to_string(CONFIG_FILE).await.unwrap()
        }
    };

    // Split up the config file into lines and filter out comments
    let lines: Vec<String> = config
        .split('\n')
        .filter(|x| !x.starts_with("#"))
        .map(|x| x.to_string())
        .collect();
    println!(" -> CONFIG: {:?}", lines);

    // Loops through the lines and sets the configurations
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
            "listen_addr" => {
                let mut addr = ADDR.lock().unwrap();
                *addr = parts[1].trim().to_string();
            }
            _ => println!(" -> CONFIG: unknown config: {}", parts[0]),
        }
    }
}

// endregion: --- Config fn's
// region: --- Page handlers

// Serves the index.html file
async fn idx_handler() -> Html<&'static str> {
    println!(" -> SERVE: index.html");
    Html(include_str!("html/index.html")) // Serves the contents of index.html
}

// Serves the overlay.html file
async fn chroma_handler() -> Html<&'static str> {
    println!(" -> SERVE: overlay.html");
    Html(include_str!("html/scoreboard/overlay.html"))
}

// Serve the teaminfo.html file
async fn upload_page_handler() -> Html<&'static str> {
    println!(" -> SERVE: teaminfo.html");
    Html(include_str!("html/logo_upload/teaminfo.html"))
}

async fn countdown_handler() -> Html<&'static str> {
    println!(" -> SERVE: countdown.html");
    Html(include_str!("html/countdown/countdown.html"))
}

// Serves the main css file
async fn css_handler() -> impl IntoResponse {
    println!(" -> SERVE: style.css");
    let body = include_str!("html/style.css");
    let body = Body::from(body);
    Response::builder()
        .header(CONTENT_TYPE, TEXT_CSS.to_string())
        .body(body)
        .unwrap()
}

// Serves the htmx library
async fn htmx_handler() -> impl IntoResponse {
    println!(" -> SERVE: htmx.min.js");
    let body = include_str!("html/htmx.min.js");
    let body = Body::from(body);
    Response::builder()
        .header(CONTENT_TYPE, TEXT_JAVASCRIPT.to_string())
        .body(body)
        .unwrap()
}

async fn app_js_handler() -> impl IntoResponse {
    println!(" -> SERVE: htmx.min.js");
    let body = include_str!("app.js");
    let body = Body::from(body);
    Response::builder()
        .header(CONTENT_TYPE, TEXT_JAVASCRIPT.to_string())
        .body(body)
        .unwrap()
}

async fn favicon_handler() -> impl IntoResponse {
    println!(" -> SERVE: favicon.ico");
    let body = include_bytes!("html/favicon.png");
    let body = Body::from(body.to_vec());
    Response::builder()
        .header(CONTENT_TYPE, "image/x-icon".to_string())
        .body(body)
        .unwrap()
}

// endregion: --- Page handlers
// region: --- Team names

// Struct to hold the team names
#[derive(Deserialize)]
struct UpdNames {
    home: String,
    away: String,
}

// Handles the form to update the team names
async fn tname_handler(Form(names): Form<UpdNames>) {
    println!(" -> TEAMS: update names: {} - {}", names.home, names.away);
    let mut home_name = HOME_NAME.lock().unwrap();
    let mut away_name = AWAY_NAME.lock().unwrap();
    *home_name = names.home;
    *away_name = names.away;
}

// Handles the display of the home team's name
async fn hdisp_handler() -> Html<String> {
    let home_name = HOME_NAME.lock().unwrap();
    Html(format!("<h2>Home: {}</h2>", home_name))
}

// Handles the display of the away team's name
async fn adisp_handler() -> Html<String> {
    let away_name = AWAY_NAME.lock().unwrap();
    Html(format!("<h2>Away: {}</h2>", away_name))
}

// Handles the display of the home team's name for the scoreboard
async fn hname_scoreboard_handler() -> Html<String> {
    let home_name = HOME_NAME.lock().unwrap();
    Html(format!("{}", home_name))
}

// Handles the display of the away team's name for the scoreboard
async fn aname_scoreboard_handler() -> Html<String> {
    let away_name = AWAY_NAME.lock().unwrap();
    Html(format!("{}", away_name))
}

// Handles and returns requests for the home team's logo
async fn home_img_handler() -> impl IntoResponse {
    let home_image = HOME_IMG_DATA.lock().unwrap().clone();
    let body = Body::from(home_image);
    Response::builder()
        .header(CONTENT_TYPE, IMAGE_PNG.to_string())
        .body(body)
        .unwrap()
}

// Handles and returns requests for the away team's logo
async fn away_img_handler() -> impl IntoResponse {
    let away_image = AWAY_IMG_DATA.lock().unwrap().clone();
    let body = Body::from(away_image);
    Response::builder()
        .header(CONTENT_TYPE, IMAGE_PNG.to_string())
        .body(body)
        .unwrap()
}

// endregion: --- Team names
// region: --- Home handlers

// Increases the home team's points by 1
async fn hu_handler() {
    // Increments home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    *home_points += 1;
}

// Decreases the home team's points by 1
async fn hd_handler() {
    // Decrements home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    if *home_points > 0 {
        *home_points -= 1;
    }
}

// Increases the home team's points by 2
async fn hu2_handler() {
    // Adds 2 home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    *home_points += 2;
}

// Increases the home team's points by 3
async fn hu3_handler() {
    // Adds 3 home points
    let mut home_points = HOME_POINTS.lock().unwrap();
    *home_points += 3;
}

// Handles and returns the home team's points
async fn hp_handler() -> Html<String> {
    // Displays home points
    let home_points = HOME_POINTS.lock().unwrap();
    Html(format!("{}", *home_points))
}

// endregion: --- Home handlers
// region: --- Away handlers

// Increases the away team's points by 1
async fn au_handler() {
    // Increments home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    *away_points += 1;
}

// Decreases the away team's points by 1
async fn ad_handler() {
    // Decrements home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    if *away_points > 0 {
        *away_points -= 1;
    }
}

// Increases the away team's points by 2
async fn au2_handler() {
    // Adds 2 home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    *away_points += 2;
}

// Increases the away team's points by 3
async fn au3_handler() {
    // Adds 3 home points
    let mut away_points = AWAY_POINTS.lock().unwrap();
    *away_points += 3;
}

// Handles and returns the away team's points
async fn ap_handler() -> Html<String> {
    // Displays home points
    let away_points = AWAY_POINTS.lock().unwrap();
    Html(format!("{}", *away_points))
}

// endregion: --- Away Handlers
// region: --- Clock handlers

// Sets the clock to 8 minutes
async fn quick_time8_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 8;
    *time_secs = 0;
}

// Sets the clock to 5 minutes
async fn quick_time5_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 5;
    *time_secs = 0;
}

// Sets the clock to 3 minutes
async fn quick_time3_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 3;
    *time_secs = 0;
}

// Sets the clock to 1 minute
async fn quick_time1_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 1;
    *time_secs = 0;
}

// Handles and returns the time formatted as "mm:ss"
async fn time_handler() -> Html<String> {
    let time_mins = TIME_MINS.lock().unwrap();
    let time_secs = TIME_SECS.lock().unwrap();
    Html(format!("{}:{:02?}", *time_mins, *time_secs))
}

// Handles and returns the minutes of the time
async fn dashboard_time_display_handler() -> Html<String> {
    Html(format!(
        "{}:{:02?}",
        *TIME_MINS.lock().unwrap(),
        *TIME_SECS.lock().unwrap()
    ))
}

// Increases the minutes of the time by 1
async fn mins_up_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    *time_mins += 1;
}

// Decreases the minutes of the time by 1
async fn mins_down_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    if *time_mins > 0 {
        *time_mins -= 1;
    }
}

// Increases the seconds of the time by 1
async fn secs_up_handler() {
    let mut time_secs = TIME_SECS.lock().unwrap();
    if *time_secs < 59 {
        *time_secs += 1;
    } else {
        let mut time_mins = TIME_MINS.lock().unwrap();
        *time_mins = *time_mins + 1;
        *time_secs = 0;
    }
}

// Decreases the seconds of the time by 1
async fn secs_down_handler() {
    let mut time_secs = TIME_SECS.lock().unwrap();
    let mut time_mins = TIME_MINS.lock().unwrap();
    if *time_secs > 0 {
        *time_secs -= 1;
    } else if *time_mins - 1 > 0 {
        *time_mins = *time_mins - 1;
        *time_secs = 59;
    }
}

// Ticks the clock down if the clock is not stopped
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

// Starts the clock
async fn tstart_handler() {
    println!(" -> TIMER: start");
    let mut time_started = TIME_STARTED.lock().unwrap();
    *time_started = true;
}

// Stops the clock
async fn tstop_handler() {
    println!(" -> TIMER: stop");
    let mut time_started = TIME_STARTED.lock().unwrap();
    *time_started = false;
}

// endregion: --- Clock handlers
// region: --- Quarter handlers

// Handles and returns the current quarter formatted for the scoreboard
async fn quarter_handler() -> Html<&'static str> {
    let quarter = QUARTER.lock().unwrap();
    if *SHOW_QUARTER.lock().unwrap() {
        if *quarter == 1 {
            return Html("1st");
        } else if *quarter == 2 {
            return Html("2nd");
        } else if *quarter == 3 {
            return Html("3rd");
        } else if *quarter == 4 {
            return Html("4th");
        } else {
            return Html("OVERTIME");
        }
    } else {
        return Html("");
    }
}

// Handles the show quarter button
async fn quarter_show_handler() {
    let mut show_quarter = SHOW_QUARTER.lock().unwrap();
    if *show_quarter {
        *show_quarter = false;
    } else {
        *show_quarter = true;
    }
}

// Handles and returns the css for the show quarter button
async fn show_quarter_css_handler() -> Html<&'static str> {
    let show_quarter = SHOW_QUARTER.lock().unwrap();
    if *show_quarter {
        return Html("<style> #show-quarter { background-color: rgb(227, 45, 32); } </style>");
    } else {
        return Html("<style> #show-quarter { background-color: #e9981f; } </style>");
    }
}

// Changes the quarter to 1
async fn quarter_change_handler(axum::extract::Path(q): axum::extract::Path<u8>) {
    let mut quarter = QUARTER.lock().unwrap();
    *quarter = q;
}

// endregion: --- Quarter handlers
// region: --- Team preset handlers

#[derive(Serialize, Deserialize, Debug)]
struct TeamInfoContainer {
    home_name: String,
    home_color: String,
    away_name: String,
    away_color: String,
}

async fn team_selectors_handler() -> Html<String> {
    let mut inject_html = String::new();
    let mut team_presets = tokio::fs::read_dir("./teams").await.unwrap();
    let mut valid_ids: Vec<String> = Vec::new();

    while let Ok(Some(res)) = team_presets.next_entry().await {
        if !res.file_name().to_string_lossy().to_string().contains(".") {
            valid_ids.push(res.file_name().into_string().unwrap());
        }
    }

    for i in &valid_ids {
        let team_info_json = tokio::fs::read_to_string(format!("./teams/{}/teaminfo.json", i))
            .await
            .expect("Id doesnt exist!");
        let team_info: TeamInfoContainer =
            serde_json::from_str(&team_info_json).expect("Could not deserialize data!");

        let home_img_bytes = tokio::fs::read(format!("./teams/{}/home.png", i))
            .await
            .unwrap();
        let away_img_bytes = tokio::fs::read(format!("./teams/{}/away.png", i))
            .await
            .unwrap();

        inject_html += &format!(
            "
            <div class=\"match-selector\">
                <p>{} vs. {}</p>
                <div style=\"display: inline\">
                    <img src=\"data:image/png;base64,{}\" height=\"30px\" width=\"auto\"/>
                    <img src=\"data:image/png;base64,{}\" height=\"30px\" width=\"auto\"/>
                </div>
                <br>
                <button hx-post=\"/load_team/{}\" hx-swap=\"none\" style=\"width: 100%;\">Select</button>
            </div>
        ",
            team_info.home_name,
            team_info.away_name,
            BASE64_STANDARD.encode(home_img_bytes),
            BASE64_STANDARD.encode(away_img_bytes),
            i
        );
    }

    dbg!(valid_ids);

    Html::from(inject_html)
}

async fn load_team_handler(axum::extract::Path(id): axum::extract::Path<String>) {
    let team_info_json = tokio::fs::read_to_string(format!("./teams/{}/teaminfo.json", id))
        .await
        .expect("Id doesnt exist!");

    let team_info: TeamInfoContainer =
        serde_json::from_str(&team_info_json).expect("Could not deserialize data!");

    println!(" -> LOAD: match {:?}", team_info);

    *HOME_NAME.lock().unwrap() = team_info.home_name;
    *AWAY_NAME.lock().unwrap() = team_info.away_name;

    *HOME_IMG_DATA.lock().unwrap() = tokio::fs::read(format!("./teams/{}/home.png", id))
        .await
        .unwrap();
    *AWAY_IMG_DATA.lock().unwrap() = tokio::fs::read(format!("./teams/{}/away.png", id))
        .await
        .unwrap();
}

// Handles the file upload for the team's logo
async fn add_team_handler(mut payload: Multipart) -> impl IntoResponse {
    const BASE62: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    let mut home_name = String::from("");
    let mut home_color = String::from("");

    let mut away_name = String::from("");
    let mut away_color = String::from("");

    let mut id = String::with_capacity(12);
    for _ in 0..12 {
        id.push(BASE62[thread_rng().gen_range(0..BASE62.len())] as char);
    }

    std::fs::create_dir_all(Path::new(&format!("./teams/{}", id))).unwrap();

    // Loops through the fields of the form
    while let Some(field) = payload.next_field().await.unwrap() {
        // Gets the name and data of the field
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        if name == "home.png" || name == "away.png" {
            // Writes the data to a .png file
            println!(" -> LOGO: recieved {}\n\tLENGTH: {}", name, data.len());
            tokio::fs::write(Path::new(&format!("./teams/{}/{}", id, name)), data)
                .await
                .unwrap();
        } else if name == "home_name" {
            home_name = std::str::from_utf8(&data).unwrap().to_string();
        } else if name == "away_name" {
            away_name = std::str::from_utf8(&data).unwrap().to_string();
        } else if name == "home_color" {
            home_color = std::str::from_utf8(&data).unwrap().to_string();
        } else if name == "away_color" {
            away_color = std::str::from_utf8(&data).unwrap().to_string();
        }
    }

    let info_container = TeamInfoContainer {
        home_name: home_name,
        home_color: home_color,
        away_name: away_name,
        away_color: away_color,
    };

    dbg!(&info_container);

    let json = serde_json::to_string(&info_container).expect("Failed to serialize team info");
    tokio::fs::write(Path::new(&format!("./teams/{}/teaminfo.json", id)), json)
        .await
        .expect("Failed to write to team info");

    StatusCode::OK
}

// endregion: --- File upload handlers
// region: --- Sponsor roll

async fn load_sponsors() -> Vec<Html<String>> {
    let mut entries = tokio::fs::read_dir("./sponsors").await.unwrap();
    let mut sponsor_imgs: Vec<tokio::fs::DirEntry> = Vec::new();

    while let Ok(Some(res)) = entries.next_entry().await {
        let entry = res;
        if let Some(extension) = entry.path().extension() {
            if extension == "png" {
                sponsor_imgs.push(entry);
            }
        }
    }

    let mut img_tags: Vec<Html<String>> = Vec::new();

    for i in 0..sponsor_imgs.len() {
        let img_bytes = tokio::fs::read(sponsor_imgs[i].path()).await.unwrap();

        img_tags.push(Html(format!(
            "<img src=\"data:image/png;base64,{}\" width=\"10%\" height=\"10%\" id=\"sponsor_roll_img\"/>",
            BASE64_STANDARD.encode(&img_bytes)
        )));
    }

    return img_tags;
}

async fn sponsor_roll_ticker() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let last_sponsor = LAST_SPONSOR.load(Ordering::SeqCst);

        if last_sponsor + 1 > SPONSOR_IMG_TAGS.lock().unwrap().len() - 1 {
            LAST_SPONSOR.store(0, Ordering::SeqCst);
        } else {
            LAST_SPONSOR.fetch_add(1, Ordering::SeqCst);
        }
    }
}

async fn sponsor_roll_handler() -> Html<String> {
    let sponsor_imgs = SPONSOR_IMG_TAGS.lock().unwrap();
    let last_sponsor = LAST_SPONSOR.load(Ordering::SeqCst);

    sponsor_imgs[last_sponsor].clone()
}

async fn show_sponsor_roll_handler() {
    let mut show_sponsor = SHOW_SPONSOR.lock().unwrap();

    if *show_sponsor {
        *show_sponsor = false;
    } else {
        *show_sponsor = true;
    }
}

async fn sponsor_roll_css_handler() -> Html<&'static str> {
    let show_sponsor = SHOW_SPONSOR.lock().unwrap();

    if *show_sponsor {
        return Html("<style> #show-sponsor { background-color: rgb(227, 45, 32); } </style>");
    } else {
        return Html("<style> #sponsor_roll_img { display: none; } #show-sponsor { background-color: #e9981f; } </style>");
    }
}

// endregion: --- Sponsor roll
// region: --- Countdown

async fn countdown_ticker() {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut countdown_started = COUNTDOWN_STARTED.lock().unwrap();
        if *countdown_started {
            let mut countdown_mins = COUNTDOWN_MINS.lock().unwrap();
            let mut countdown_secs = COUNTDOWN_SECS.lock().unwrap();
            if *countdown_secs == 0 {
                if *countdown_mins == 0 {
                    *countdown_started = false;
                } else {
                    *countdown_mins -= 1;
                    *countdown_secs = 59;
                }
            } else {
                *countdown_secs -= 1;
            }
        }
    }
}

async fn start_countdown_handler() {
    *COUNTDOWN_STARTED.lock().unwrap() = true;
}

async fn stop_countdown_handler() {
    *COUNTDOWN_STARTED.lock().unwrap() = false;
}

async fn countdown_display_handler() -> Html<String> {
    Html(format!(
        "<h2>{}</h2> <br>
         {}:{:02?}
    ",
        *COUNTDOWN_TITLE.lock().unwrap(),
        *COUNTDOWN_MINS.lock().unwrap(),
        *COUNTDOWN_SECS.lock().unwrap()
    ))
}

async fn dashboard_countdown_display_handler() -> Html<String> {
    Html(format!(
        "{}:{:02?}",
        *COUNTDOWN_MINS.lock().unwrap(),
        *COUNTDOWN_SECS.lock().unwrap()
    ))
}

async fn show_countdown_handler() {
    let mut show_countdown = SHOW_COUNTDOWN.lock().unwrap();
    if *show_countdown {
        *show_countdown = false;
    } else {
        *show_countdown = true;
    }
}

async fn countdown_css_handler() -> Html<&'static str> {
    if *SHOW_COUNTDOWN.lock().unwrap() {
        return Html("<style> .white-boxes-container { display: none; } #show-countdown { background-color: rgb(227, 45, 32); } </style>");
    } else {
        return Html("<style> .white-boxes-container { display: flex; } #show-countdown { background-color: #e9981f; } #countdown { display: none; }</style>");
    }
}

async fn qtc20_handler() {
    *COUNTDOWN_MINS.lock().unwrap() = 20;
    *COUNTDOWN_SECS.lock().unwrap() = 0;
}

async fn qtc15_handler() {
    *COUNTDOWN_MINS.lock().unwrap() = 15;
    *COUNTDOWN_SECS.lock().unwrap() = 0;
}

async fn qtc10_handler() {
    *COUNTDOWN_MINS.lock().unwrap() = 10;
    *COUNTDOWN_SECS.lock().unwrap() = 0;
}

async fn qtc5_handler() {
    *COUNTDOWN_MINS.lock().unwrap() = 5;
    *COUNTDOWN_SECS.lock().unwrap() = 0;
}

async fn countdown_mins_up_handler() {
    let mut countdown_mins = COUNTDOWN_MINS.lock().unwrap();
    *countdown_mins = *countdown_mins + 1;
}

async fn countdown_mins_down_handler() {
    let mut countdown_mins = COUNTDOWN_MINS.lock().unwrap();
    if *countdown_mins > 0 {
        *countdown_mins = *countdown_mins - 1;
    }
}

async fn countdown_secs_up_handler() {
    let mut countdown_secs = COUNTDOWN_SECS.lock().unwrap();
    if *countdown_secs < 59 {
        *countdown_secs = *countdown_secs + 1;
    } else {
        let mut countdown_mins = COUNTDOWN_MINS.lock().unwrap();
        *countdown_mins = *countdown_mins + 1;
        *countdown_secs = 0;
    }
}

async fn countdown_secs_down_handler() {
    let mut countdown_secs = COUNTDOWN_SECS.lock().unwrap();
    let mut countdown_mins = COUNTDOWN_MINS.lock().unwrap();
    if *countdown_secs > 0 {
        *countdown_secs = *countdown_secs - 1;
    } else if *countdown_mins - 1 > 0 {
        *countdown_mins = *countdown_mins - 1;
        *countdown_secs = 59;
    }
}

#[derive(Deserialize)]
struct CountdownTitle {
    title: String,
}

async fn countdown_title_handler(Form(title_data): Form<CountdownTitle>) -> Redirect {
    println!(" -> COUNTDOWN: title set to {}", title_data.title);
    *COUNTDOWN_TITLE.lock().unwrap() = title_data.title;
    Redirect::to("/countdown")
}

// endregion: --- Countdown
// region: --- Misc handelers

// Function for testing http requests
//async fn test_handler() {
//    println!(" -> TEST: test");
//}

// Handles and returns the chromakey color as a css background color
async fn chromargb_handler() -> Html<String> {
    let chromakey = CHROMAKEY.lock().unwrap();
    Html(format!(
        "<style>body {{ background-color: rgb({}, {}, {}); }}</style>",
        chromakey.0, chromakey.1, chromakey.2
    ))
}

// Handles and returns the score as a string formatted for the scoreboard
async fn score_handler() -> Html<String> {
    let home_points = HOME_POINTS.lock().unwrap();
    let away_points = AWAY_POINTS.lock().unwrap();
    Html(format!("{} - {}", home_points, away_points))
}

// Handles and returns the time and quarter as a string formatted for the scoreboard
async fn time_and_quarter_handler() -> Html<String> {
    let time_mins = TIME_MINS.lock().unwrap();
    let time_secs = TIME_SECS.lock().unwrap();
    let quarter = QUARTER.lock().unwrap();
    let show_quarter = SHOW_QUARTER.lock().unwrap();
    if *show_quarter {
        if *quarter == 1 {
            return Html(format!("{}:{:02?} - 1st", time_mins, time_secs));
        } else if *quarter == 2 {
            return Html(format!("{}:{:02?} - 2nd", time_mins, time_secs));
        } else if *quarter == 3 {
            return Html(format!("{}:{:02?} - 3rd", time_mins, time_secs));
        } else if *quarter == 4 {
            return Html(format!("{}:{:02?} - 4th", time_mins, time_secs));
        } else {
            return Html(format!("{}:{:02?} - OVERTIME", time_mins, time_secs));
        }
    } else {
        return Html(format!("{}:{:02?}", time_mins, time_secs));
    }
}

async fn reset_scoreboard_handler() {
    println!(" -> SCOREBOARD: reset");
    *HOME_NAME.lock().unwrap() = String::from("team_name");
    *AWAY_NAME.lock().unwrap() = String::from("team_name");

    *HOME_POINTS.lock().unwrap() = 0;
    *AWAY_POINTS.lock().unwrap() = 0;

    *TIME_MINS.lock().unwrap() = 0;
    *TIME_SECS.lock().unwrap() = 0;

    *TIME_STARTED.lock().unwrap() = false;

    *QUARTER.lock().unwrap() = 1;

    *COUNTDOWN_TITLE.lock().unwrap() = String::from("countdown_title");

    *COUNTDOWN_MINS.lock().unwrap() = 0;
    *COUNTDOWN_SECS.lock().unwrap() = 0;
    *COUNTDOWN_STARTED.lock().unwrap() = false;
}

// endregion: -- Sponsor roll

// region: --- Misc fn's

// endregion: --- Misc fn's
