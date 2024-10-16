#[forbid(unsafe_code)]
mod appstate;
mod routing;

use crate::appstate::global::*;
use crate::appstate::routing::*;

use crate::routing::basic::*;
use crate::routing::login::*;
use crate::routing::overlay::*;
use crate::routing::sponsors::*;
use crate::routing::team::*;
use crate::routing::teaminfo::*;
use crate::routing::time::*;
use crate::routing::visibility::*;
use crate::routing::websockets::*;
use crate::routing::downs::*;
use crate::routing::api::*;
use crate::routing::updating::*;

use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{
        header::{LOCATION, SET_COOKIE},
        HeaderMap, HeaderValue, StatusCode,
    },
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, head, post, put},
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use base64::prelude::*;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    time::{Instant, UNIX_EPOCH},
};
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
    signal,
    sync::oneshot,
};
use tower_http::cors::CorsLayer;

#[macro_export]
macro_rules! printlg {
    ($($arg:tt)*) => {{
        use std::fmt::Write;

        let mut buffer = String::new();
        write!(&mut buffer, $($arg)*).expect("Failed to write to buffer");

        LOGS.lock().await.push(buffer.clone());

        println!("{}", buffer);
    }};
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    // Verify tmp directory exists
    create_dir_all("./tmp").await?;

    // Wait for program lock to release
    if std::path::Path::new("./tmp/froggi.lock").exists() {
        printlg!("Waiting on program lock to release...");

        loop {
            if std::path::Path::new("./tmp/froggi.lock").exists() {
                let lock_timestamp = tokio::fs::read_to_string("./tmp/froggi.lock").await?;
                let current_time = std::time::SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards")
                    .as_secs();

                if current_time - lock_timestamp.trim().parse::<u64>()? >= 30 {
                    printlg!("Lock not updated for 30 seconds, old lock assumed to have crashed.");
                    break;
                }
            } else {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    program_lock().await?;

    let (restart_tx, restart_rx) = oneshot::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (update_tx, update_rx) = oneshot::channel();

    // Initialize the application state
    let mut state = AppState::default();

    if let Ok(s) = AppState::load_saved_state().await {
        state = s;
    }

    *RESTART_SIGNAL.lock().await = Some(restart_tx);
    *SHUTDOWN_SIGNAL.lock().await = Some(shutdown_tx);
    *UPDATE_SIGNAL.lock().await = Some(update_tx);

    // Validate required files and directories
    if let Err(_) = File::open("secret.key").await {
        printlg!("Initializing secret.key");
        let mut f = File::create("secret.key").await?;

        let key: [u8; 32] = rand::thread_rng().gen();
        let secret = BASE64_STANDARD.encode(key);

        f.write_all(secret.as_bytes()).await?;
    }

    match File::open("config.json").await {
        Ok(_) => {
            let cfg: Config =
                serde_json::from_str(&tokio::fs::read_to_string("config.json").await?)?;

            if cfg.secure_auth_cookie == false {
                printlg!("! ! ! ! ! ! ! ! ! !");
                printlg!("WARNING! DISABLING SECURE AUTH COOKIE IN config.json COULD RESULT IN SENDING LOGIN CREDENTIALS OVER UNENCRYPTED TRAFFIC, THIS IS UNSAFE AND SHOULD ONLY BE USED FOR DEVELOPMENT PURPOSES! UNLESS YOU KNOW WHAT YOU ARE DOING, PLEASE ENABLE SECURE AUTH COOKIE.");
                printlg!("! ! ! ! ! ! ! ! ! !");
            }
        }
        Err(_) => {
            printlg!("Initializing config.json");
            let mut f = File::create("config.json").await?;

            let default_config = Config {
                secure_auth_cookie: true,
                sponsor_wait_time: 5,
                countdown_opacity: 0.5,
            };

            f.write_all(serde_json::to_string_pretty(&default_config)?.as_bytes())
                .await?;
        }
    }

    create_dir_all(format!("./sponsors")).await?;

    create_dir_all(format!("./team-presets")).await?;

    // Load sponsor img tags
    load_sponsors().await;
    load_config().await;

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any) // Allow requests from any origin
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::HEAD,
        ]) // Allow specific methods
        .allow_headers(tower_http::cors::Any)
        .allow_private_network(true);

    let auth_give_session_routes = Router::new()
        .route("/", get(index_handler))
        .route("/teaminfo", get(teaminfo_handler))
        .route("/settings", get(settings_handler))
        .layer(middleware::from_fn(auth_give_session_layer));

    let auth_session_routes = Router::new()
        .route("/dashboard-websocket", get(dashboard_websocket_handler))
        .route("/home-points/update/:a", post(home_points_update_handler))
        .route("/home-points/set/:a", post(home_points_set_handler))
        .route("/away-points/update/:a", post(away_points_update_handler))
        .route("/away-points/set/:a", post(away_points_set_handler))
        .route("/game-clock/ctl/:o", post(game_clock_ctl_handler))
        .route("/game-clock/set/:mins", post(game_clock_set_handler))
        .route(
            "/game-clock/update/:mins/:secs",
            post(game_clock_update_handler),
        )
        .route("/countdown-clock/ctl/:o", post(countdown_clock_ctl_handler))
        .route(
            "/countdown-clock/set/:mins",
            post(countdown_clock_set_handler),
        )
        .route(
            "/countdown-clock/update/:mins/:secs",
            post(countdown_clock_update_handler),
        )
        .route("/countdown/text/set", post(countdown_text_set_handler))
        .route("/quarter/set/:q", post(quarter_set_handler))
        .route("/quarter/update/:a", post(quarter_update_handler))
        .route("/teaminfo/create", post(teaminfo_preset_create_handler))
        .route("/teaminfo/select/:id", post(teaminfo_preset_select_handler))
        .route("/teaminfo/remove/:id", post(teaminfo_preset_remove_handler))
        .route(
            "/sponsors/upload",
            post(upload_sponsors_handler).layer(DefaultBodyLimit::max(2000000000)),
        )
        .route("/sponsors/remove/:id", post(sponsor_remove_handler))
        .route("/downs/set/:d", post(downs_set_handler))
        .route("/downs/update/:d", post(downs_update_handler))
        .route("/downs/togo/set/:y", post(downs_togo_set_handler))
        .route("/downs/togo/update/:y", post(downs_togo_update_handler))
        .route("/visibility/toggle/:v", post(visibility_toggle_handler))
        .route("/ocr/api/toggle", post(ocr_api_toggle_handler))
        .route("/api/key/show", put(api_key_show_handler))
        .route("/api/key/reveal", post(api_key_reveal_handler))
        .route("/api/key/regen", post(api_key_regen_handler))
        .route("/popup/:t", post(popup_handler))
        .route("/reset", post(reset_handler))
        .route("/restart", post(restart_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/update", post(update_handler))
        .route("/logout", post(logout_handler))
        .layer(middleware::from_fn(auth_session_layer));

    let app = Router::new()
        .route("/", head(ping_handler))
        .route("/overlay", get(overlay_handler))
        .route("/styles.css", get(css_handler))
        .route("/htmx.js", get(htmx_js_handler))
        .route("/app.js", get(app_js_handler))
        .route("/ws.js", get(ws_js_handler))
        .route("/favicon.png", get(favicon_handler))
        .route("/overlay-websocket", get(overlay_websocket_handler))
        .route("/login", get(login_page_handler))
        .route("/login/", get(login_page_handler))
        .route("/login", post(login_handler))
        .route("/login/create", get(create_login_page_handler))
        .route("/login/create", post(create_login_handler))
        .route("/home-points/display", get(home_points_display_handler))
        .route("/away-points/display", get(away_points_display_handler))
        .route("/game-clock/display/:o", get(game_clock_display_handler))
        .route(
            "/countdown-clock/display/:o",
            get(countdown_clock_display_handler),
        )
        .route("/quarter/display", get(quarter_display_handler))
        .route("/teaminfo/selector", put(teaminfo_preset_selector_handler))
        .route("/teaminfo/name/:t", put(team_name_display_handler))
        .route("/teaminfo/button-css", put(teaminfo_button_css_handler))
        .route("/sponsors/manage", put(sponsors_management_handler))
        .route("/icon/:t", put(icon_handler))
        .route(
            "/overlay/team-border-css",
            put(overlay_team_border_css_handler),
        )
        .route("/downs/display/:t", get(downs_display_handler))
        .route("/visibility/buttons", put(visibility_buttons_handler))
        .route("/ocr", post(ocr_handler))
        .route("/ocr/api/button", put(ocr_api_button_handler))
        .route("/api/key/check/:k", post(api_key_check_handler))
        .route("/logs", put(logs_handler))
        .route(
            "/version",
            put(|| async { Html::from(env!("CARGO_PKG_VERSION")) }),
        )
        .route("/update/menu", put(update_menu_handler))
        .nest("/", auth_session_routes)
        .nest("/", auth_give_session_routes)
        .with_state(state.clone())
        .fallback(get(not_found_handler))
        .layer(cors);

    if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        tokio::spawn(uptime_ticker());
        tokio::spawn(update_program_lock());
        tokio::spawn(game_clock_ticker());
        tokio::spawn(countdown_clock_ticker());
        tokio::spawn(sponsor_ticker());
        tokio::spawn(popup_home_ticker());
        tokio::spawn(popup_away_ticker());
        tokio::spawn(auto_update_checker());
        printlg!(" -> LISTENING ON: 0.0.0.0:3000");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(restart_rx, shutdown_rx, update_rx))
            .await
            .context("Could not serve app")?;
    } else {
        panic!("Could not bind tcp listener!");
    }

    printlg!("Saving app state...");

    if let Ok(save_app_state) =
        serde_json::to_string(&AppStateSerde::consume_app_state(state).await)
    {
        if let Ok(_) = tokio::fs::write("./appstate.json", save_app_state).await {
            printlg!("Saved app state!");
        } else {
            printlg!("Failed to save app state!");
        }
    } else {
        printlg!("Failed to save app state!");
    }

    release_program_lock().await?;

    if let Some(code) = EXIT_CODE.lock().await.take() {
        if code == 10 {
            printlg!("Shut down gracefully\n");
        } else {
            printlg!("Shut down gracefully");
        }
        std::process::exit(code);
    }

    printlg!("Shut down gracefully");

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Config {
    secure_auth_cookie: bool,
    sponsor_wait_time: u64,
    countdown_opacity: f32,
}

// region: basic pages

// endregion: basic pages
// region: page websockets

// endregion: page websockets
// region: js routing

// endregion: js routing
// region: image routing

// endregion: image routing
// region: login

// endregion: login
// region: team routing

// endregion: team routing
// region: time

// endregion: time
// region: quarters

// endregion: quarters
// region: teaminfo

// endregion: teaminfo
// region: sponsors

// endregion: sponsors
// region: overlay

// endregion: overlay
// region: downs

// endregion: downs
// region: visibility

// endregion: visibility
// region: api

// endregion: api
// region: updating

// endregion: updating
// region: popups

async fn popup_handler(
    Path(a): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    if let Some(p) = params.get("text") {
        if a == "home" {
            POPUPS_HOME.lock().await.push((p.clone(), 7));
        } else if a == "away" {
            POPUPS_AWAY.lock().await.push((p.clone(), 7));
        }
        printlg!("POPUP: {}", p);
    }

    return StatusCode::OK;
}

async fn popup_home_ticker() {
    loop {
        let start_time = Instant::now();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut popups = POPUPS_HOME.lock().await;

        let mut i = 0;
        loop {
            if i >= popups.len() {
                break;
            }

            let time_diff = (Instant::now() - start_time).as_secs();
            if popups[i].1 - time_diff > 0 {
                popups[i].1 -= time_diff;
                i += 1;
            } else {
                popups.remove(i);
            }
        }
    }
}

async fn popup_away_ticker() {
    loop {
        let start_time = Instant::now();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut popups = POPUPS_AWAY.lock().await;

        let mut i = 0;
        loop {
            if i >= popups.len() {
                break;
            }

            let time_diff = (Instant::now() - start_time).as_secs();
            if popups[i].1 - time_diff > 0 {
                popups[i].1 -= time_diff;
                i += 1;
            } else {
                popups.remove(i);
            }
        }
    }
}

// endregion: popups
// region: misc

async fn reset_handler(State(ref mut state): State<AppState>) -> impl IntoResponse {
    *state.home_points.lock().await = 0;
    *state.home_name.lock().await = String::from("Home");
    *state.away_points.lock().await = 0;
    *state.away_name.lock().await = String::from("Away");
    *state.quarter.lock().await = 1;
    *state.preset_id.lock().await = String::new();
    *state.down.lock().await = 1;
    *state.downs_togo.lock().await = 1;
    *state.countdown_text.lock().await = String::from("Countdown");
    *state.show_countdown.lock().await = false;
    *state.show_downs.lock().await = true;
    *state.show_scoreboard.lock().await = true;

    *GAME_CLOCK.lock().await = 0;
    *GAME_CLOCK_START.lock().await = false;
    *COUNTDOWN_CLOCK.lock().await = 0;
    *COUNTDOWN_CLOCK_START.lock().await = false;
    *SHOW_SPONSORS.lock().await = false;
    *OCR_API.lock().await = false;

    printlg!("SCOREBOARD REST");

    return StatusCode::OK;
}

async fn logs_handler() -> impl IntoResponse {
    let logs = LOGS.lock().await;
    let mut logs_display = Vec::new();

    for i in 0..logs.len() {
        logs_display.push(format!("<span>({}) {}</span>", i + 1, logs[i]))
    }

    Html::from(logs_display.join("<br>"))
}

async fn load_config() {
    let config: Config = serde_json::from_str(
        &tokio::fs::read_to_string("./config.json")
            .await
            .expect("Failed to read config.json"),
    )
    .expect("Failed to deserialize config.json");

    *COUNTDOWN_OPACITY.lock().await = config.countdown_opacity;
}

fn hex_to_rgb(hex: &String) -> (u8, u8, u8) {
    let hex_chars: Vec<char> = hex.trim_start_matches("#").to_string().chars().collect();

    let r = hex_char_to_u8(hex_chars[0]) * 16 + hex_char_to_u8(hex_chars[1]);
    let g = hex_char_to_u8(hex_chars[2]) * 16 + hex_char_to_u8(hex_chars[3]);
    let b = hex_char_to_u8(hex_chars[4]) * 16 + hex_char_to_u8(hex_chars[5]);

    (r, g, b)
}

fn hex_char_to_u8(c: char) -> u8 {
    match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' => 10,
        'B' => 11,
        'C' => 12,
        'D' => 13,
        'E' => 14,
        'F' => 15,
        'a' => 10,
        'b' => 11,
        'c' => 12,
        'd' => 13,
        'e' => 14,
        'f' => 15,
        _ => 15,
    }
}

fn rgb_to_hex(rgb: &(u8, u8, u8)) -> String {
    format!(
        "#{}{}{}{}{}{}",
        u8_to_hex_char((rgb.0 - (rgb.0 % 16)) / 16),
        u8_to_hex_char(rgb.0 % 16),
        u8_to_hex_char((rgb.1 - (rgb.1 % 16)) / 16),
        u8_to_hex_char(rgb.1 % 16),
        u8_to_hex_char((rgb.2 - (rgb.2 % 16)) / 16),
        u8_to_hex_char(rgb.2 % 16)
    )
}

fn u8_to_hex_char(u: u8) -> char {
    match u {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'A',
        11 => 'B',
        12 => 'C',
        13 => 'D',
        14 => 'E',
        15 => 'F',
        _ => 'F',
    }
}

async fn restart_handler() -> impl IntoResponse {
    printlg!("Restarting...");

    if let Some(tx) = RESTART_SIGNAL.lock().await.take() {
        let _ = tx.send(());
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from("Restarting..."))
            .unwrap();
    } else {
        printlg!("Restart signal already sent");
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(String::from("Restart already sent!"))
            .unwrap();
    }
}

async fn shutdown_handler() -> impl IntoResponse {
    printlg!("Shutting down...");

    if let Some(tx) = SHUTDOWN_SIGNAL.lock().await.take() {
        let _ = tx.send(());
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from("Shutting down..."))
            .unwrap();
    } else {
        printlg!("Shutdown signal already sent");
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(String::from("Shutdown already sent!"))
            .unwrap();
    }
}

async fn ping_handler() -> impl IntoResponse {
    return StatusCode::OK;
}

// endregion: misc
// region: middleware

async fn auth_session_layer(
    jar: CookieJar,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if verify_session(jar).await {
        return Ok(next.run(request).await);
    } else {
        if let Some(h) = headers.get("api-auth") {
            let login: Login = serde_json::from_str(
                &tokio::fs::read_to_string("./login.json")
                    .await
                    .expect("Failed to read login.json"),
            )
            .expect("Failed to deserialize login.json");
            if h.to_str().expect("Failed to cast headervalue into a str") == login.api_key {
                return Ok(next.run(request).await);
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
}

async fn auth_give_session_layer(
    jar: CookieJar,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    if verify_auth(jar.clone()).await || verify_session(jar.clone()).await {
        if verify_session(jar).await {
            return Ok(next.run(request).await);
        } else {
            let mut response = next.run(request).await;
            let cookie = session_cookie_builder().await;

            response.headers_mut().append(
                SET_COOKIE,
                HeaderValue::from_str(&cookie)
                    .expect("Failed to get headervalue from session cookie!"),
            );

            return Ok(response);
        }
    } else {
        return Err(Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login"))
            .body(Body::empty())
            .unwrap());
    }
}

// endregion: middleware

fn id_create(l: u8) -> String {
    const BASE62: &'static str = "qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890";

    let mut id = String::new();
    let base62: Vec<char> = BASE62.chars().collect();

    for _ in 0..l {
        id.push(base62[thread_rng().gen_range(0..base62.len())])
    }

    id
}

async fn program_lock() -> Result<(), std::io::Error> {
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    tokio::fs::write("./tmp/froggi.lock", time.to_string()).await
}

async fn release_program_lock() -> Result<(), std::io::Error> {
    tokio::fs::remove_file("./tmp/froggi.lock").await
}

async fn update_program_lock() {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        tokio::fs::write("./tmp/froggi.lock", time.to_string())
            .await
            .expect("Failed to update froggi.lock");
    }
}

// Code borrowed from https://github.com/tokio-rs/axum/blob/806bc26e62afc2e0c83240a9e85c14c96bc2ceb3/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal(
    mut restart_rx: oneshot::Receiver<()>,
    mut shutdown_rx: oneshot::Receiver<()>,
    mut update_rx: oneshot::Receiver<()>,
) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = async {
        let _ = std::future::pending::<()>().await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = &mut restart_rx => {
            *EXIT_CODE.lock().await = Some(10);
        }
        _ = &mut shutdown_rx => {
            *EXIT_CODE.lock().await = Some(0);
        }
        _ = &mut update_rx => {
            *EXIT_CODE.lock().await = Some(11);
        }
    }
}
