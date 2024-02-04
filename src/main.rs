use axum::{
    body::Body,
    extract::Multipart,
    http::Response,
    response::{Html, IntoResponse},
    routing::{get, head, post, put},
    Form, Router,
};

use lazy_static::lazy_static;
use std::{path::Path, sync::Mutex};

use hyper::{header::CONTENT_TYPE, StatusCode};
use mime::IMAGE_PNG;
use mime::TEXT_CSS;
use mime::TEXT_JAVASCRIPT;

use serde::Deserialize;

use std::io::{self, BufRead};

const CONFIG_FILE: &'static str = "config.cfg"; // Sets the name of the config file

lazy_static! {
    static ref HOME_NAME: Mutex<String> = Mutex::new(String::from("team_name"));
    static ref AWAY_NAME: Mutex<String> = Mutex::new(String::from("team_name"));
    static ref HOME_POINTS: Mutex<i32> = Mutex::new(0);
    static ref AWAY_POINTS: Mutex<i32> = Mutex::new(0);
    static ref TIME_MINS: Mutex<i32> = Mutex::new(8);
    static ref TIME_SECS: Mutex<i32> = Mutex::new(0);
    static ref TIME_STARTED: Mutex<bool> = Mutex::new(false);
    static ref CHROMAKEY: Mutex<(u8, u8, u8)> = Mutex::new((0, 0, 0));
    static ref QUARTER: Mutex<i32> = Mutex::new(1);
    static ref SHOW_QUARTER: Mutex<bool> = Mutex::new(false);
    static ref ADDR: Mutex<String> = Mutex::new(String::from(""));
}

#[tokio::main]
async fn main() {
    // region: --- Routing

    let app = Router::new() // Creates a new router
        // Routes for the html files, css, and lib files
        .route("/", get(idx_handler)) // Handles get requests for the index of the app
        .route("/chromakey", get(chroma_handler)) // Handles get requests for the chromakey page
        .route("/upload", get(upload_page_handler)) // Handles get requests for the upload page
        .route("/style.css", get(css_handler)) // Handles get requests for the css of the app
        .route("/htmx.min.js", get(htmx_handler)) // Handles get requests for the htmx library
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
        .route("/time_secs", put(secs_handler))
        .route("/time_mins", put(mins_handler))
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
        .route("/q1", post(quarter1_change))
        .route("/q2", post(quarter2_change))
        .route("/q3", post(quarter3_change))
        .route("/q4", post(quarter4_change))
        .route("/show_quarter_css", put(show_quarter_css_handler))
        // Routes for the file upload
        .route("/logo_upload", post(logo_upload_handler))
        .route("/ping", head(|| async { StatusCode::OK }))
        // Route the 404 page
        .fallback_service(get(|| async { 
            println!(" -> 404: not found");
            ( 
                StatusCode::NOT_FOUND, 
                Html("<h1>404 - Not Found</h1>"),
            ) 
        }));

    // endregion: --- Routing

    tokio::spawn(clock_ticker());
    tokio::spawn(read_or_create_config()).await.unwrap();

    let listen_addr = ADDR.lock().unwrap();

    let listen_addr: String = listen_addr.clone();

    println!("Listening on: {}\nType \"stop\" to do shut down the server gracefully\n", listen_addr);
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap(); // Binds the listener to the address

    let (tx, rx) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            if line.unwrap() == "stop" {
                let _ = tx.send(());
                return;
            }
        }
    });

    let server = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = rx.await;
            println!(" -> SERVER: shutting down");
        });

    if let Err(err) = server.await {
        eprintln!("server error: {}", err);
    }
    println!(" -> SERVER: gracefully shut down");
}

// region: --- Config fn's

async fn read_or_create_config() {
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

    let lines: Vec<String> = config
        .split('\n')
        .filter(|x| !x.starts_with("#"))
        .map(|x| x.to_string())
        .collect();
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

async fn idx_handler() -> Html<&'static str> {
    println!(" -> SERVE: index.html");
    Html(include_str!("html/index.html")) // Serves the contents of index.html
}

async fn chroma_handler() -> Html<&'static str> {
    println!(" -> SERVE: chromakey.html");
    Html(include_str!("html/scoreboard/chromakey.html"))
}

async fn upload_page_handler() -> Html<&'static str> {
    println!(" -> SERVE: upload.html");
    Html(include_str!("html/logo_upload/upload.html"))
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

async fn tname_handler(Form(names): Form<UpdNames>) {
    println!(" -> TEAMS: update names: {} - {}", names.home, names.away);
    let mut home_name = HOME_NAME.lock().unwrap();
    let mut away_name = AWAY_NAME.lock().unwrap();
    *home_name = names.home;
    *away_name = names.away;
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

async fn quick_time8_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 8;
    *time_secs = 0;
}

async fn quick_time5_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 5;
    *time_secs = 0;
}

async fn quick_time3_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 3;
    *time_secs = 0;
}

async fn quick_time1_handler() {
    let mut time_mins = TIME_MINS.lock().unwrap();
    let mut time_secs = TIME_SECS.lock().unwrap();
    *time_mins = 1;
    *time_secs = 0;
}

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
            return Html("Q");
        }
    } else {
        return Html("");
    }
}

async fn quarter_show_handler() {
    let mut show_quarter = SHOW_QUARTER.lock().unwrap();
    if *show_quarter {
        *show_quarter = false;
    } else {
        *show_quarter = true;
    }
}

async fn show_quarter_css_handler() -> Html<&'static str> {
    let show_quarter = SHOW_QUARTER.lock().unwrap();
    if *show_quarter {
        return Html("<style> #show-quarter { background-color: rgb(227, 45, 32); } </style>");
    } else {
        return Html("<style> #show-quarter { background-color: #e32d20; } </style>");
    }
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
// region: --- File upload handlers

async fn logo_upload_handler(mut payload: Multipart) -> impl IntoResponse {
    while let Some(field) = payload.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();

        println!(" -> LOGO: recieved {}\n\tLENGTH: {}", name, data.len());
        tokio::fs::write(Path::new(&name), data).await.unwrap();

        let img = image::open(&name).unwrap();
        let img_dimensions = image::image_dimensions(&name).unwrap();

        println!(" -> IMG: {:?}", img_dimensions);

        let height = img_dimensions.1 as f32;
        let width = img_dimensions.0 as f32;

        let resize_ratio = 30.0 / height;
        println!(" -> RESIZE: {}%", resize_ratio * 100.0);

        let height: u32 = (height * resize_ratio) as u32;
        let width: u32 = (width * resize_ratio) as u32;

        println!(" -> RESIZE {}x{}", height, width);
        let resized =
            image::imageops::resize(&img, width, height, image::imageops::FilterType::Lanczos3);

        resized.save(Path::new(&name)).unwrap();

        println!(" -> RESIZE: done");
    }

    StatusCode::OK
}

// endregion: --- File upload handlers
// region: --- Misc handelers

//async fn test_handler() {
//    println!(" -> TEST: test");
//}

async fn chromargb_handler() -> Html<String> {
    let chromakey = CHROMAKEY.lock().unwrap();
    Html(format!(
        "<style>body {{ background-color: rgb({}, {}, {}); }}</style>",
        chromakey.0, chromakey.1, chromakey.2
    ))
}

async fn score_handler() -> Html<String> {
    let home_points = HOME_POINTS.lock().unwrap();
    let away_points = AWAY_POINTS.lock().unwrap();
    Html(format!("{} - {}", home_points, away_points))
}

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
            return Html(format!("{}:{:02?} - q{}", time_mins, time_secs, quarter));
        }
    } else {
        return Html(format!("{}:{:02?}", time_mins, time_secs));
    }
}

// endregion: --- Misc handelers
// region: --- Misc fn's

// endregion: --- Misc fn's
