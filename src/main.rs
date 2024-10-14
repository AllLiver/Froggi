#[forbid(unsafe_code)]
use anyhow::{anyhow, Context, Error, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    extract::{
        ws::Message, DefaultBodyLimit, Multipart, Path, Query, Request, State, WebSocketUpgrade,
    },
    http::{
        header::{CONTENT_TYPE, LOCATION, SET_COOKIE},
        HeaderMap, HeaderName, HeaderValue, StatusCode,
    },
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, head, post, put},
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::prelude::*;
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    sync::Arc,
    time::{Instant, UNIX_EPOCH},
};
use tokio::{
    fs::{create_dir_all, read_dir, remove_dir_all, remove_file, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    signal,
    sync::{oneshot, Mutex},
};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

const WEBSOCKET_UPDATE_MILLIS: u64 = 100;

lazy_static! {
    static ref UPTIME_SECS: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    static ref GAME_CLOCK: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    static ref GAME_CLOCK_START: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref COUNTDOWN_CLOCK: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    static ref COUNTDOWN_CLOCK_START: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref LOGS: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    static ref SPONSOR_TAGS: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    static ref SPONSOR_IDX: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    static ref SHOW_SPONSORS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref OCR_API: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref POPUPS_HOME: Arc<Mutex<Vec<(String, u64)>>> = Arc::new(Mutex::new(Vec::new()));
    static ref POPUPS_AWAY: Arc<Mutex<Vec<(String, u64)>>> = Arc::new(Mutex::new(Vec::new()));
    static ref COUNTDOWN_OPACITY: Arc<Mutex<f32>> = Arc::new(Mutex::new(1.0));
    static ref RESTART_SIGNAL: Arc<Mutex<Option<oneshot::Sender<()>>>> = Arc::new(Mutex::new(None));
    static ref SHUTDOWN_SIGNAL: Arc<Mutex<Option<oneshot::Sender<()>>>> =
        Arc::new(Mutex::new(None));
    static ref UPDATE_SIGNAL: Arc<Mutex<Option<oneshot::Sender<()>>>> = Arc::new(Mutex::new(None));
    static ref EXIT_CODE: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
}

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

#[derive(Clone, Debug)]
struct AppState {
    home_points: Arc<Mutex<u32>>,
    home_name: Arc<Mutex<String>>,
    away_points: Arc<Mutex<u32>>,
    away_name: Arc<Mutex<String>>,
    quarter: Arc<Mutex<u8>>,
    preset_id: Arc<Mutex<String>>,
    down: Arc<Mutex<u8>>,
    downs_togo: Arc<Mutex<u8>>,
    countdown_text: Arc<Mutex<String>>,
    show_countdown: Arc<Mutex<bool>>,
    show_downs: Arc<Mutex<bool>>,
    show_scoreboard: Arc<Mutex<bool>>,
}

impl AppState {
    fn default() -> AppState {
        AppState {
            home_points: Arc::new(Mutex::new(0)),
            home_name: Arc::new(Mutex::new(String::from("Home"))),
            away_points: Arc::new(Mutex::new(0)),
            away_name: Arc::new(Mutex::new(String::from("Away"))),
            quarter: Arc::new(Mutex::new(1)),
            preset_id: Arc::new(Mutex::new(String::new())),
            down: Arc::new(Mutex::new(1)),
            downs_togo: Arc::new(Mutex::new(1)),
            countdown_text: Arc::new(Mutex::new(String::from("Countdown"))),
            show_countdown: Arc::new(Mutex::new(false)),
            show_downs: Arc::new(Mutex::new(true)),
            show_scoreboard: Arc::new(Mutex::new(true)),
        }
    }
    async fn load_saved_state() -> Result<AppState, Error> {
        if let Ok(contents) = tokio::fs::read_to_string("./appstate.json").await {
            if let Ok(saved_state) = serde_json::from_str::<AppStateSerde>(&contents) {
                *SHOW_SPONSORS.lock().await = saved_state.show_sponsors;
                *OCR_API.lock().await = saved_state.ocr_api;
                *GAME_CLOCK.lock().await = saved_state.game_clock;
                *COUNTDOWN_CLOCK.lock().await = saved_state.countdown_clock;

                return Ok(AppState {
                    home_points: Arc::new(Mutex::new(saved_state.home_points)),
                    home_name: Arc::new(Mutex::new(saved_state.home_name)),
                    away_points: Arc::new(Mutex::new(saved_state.away_points)),
                    away_name: Arc::new(Mutex::new(saved_state.away_name)),
                    quarter: Arc::new(Mutex::new(saved_state.quarter)),
                    preset_id: Arc::new(Mutex::new(saved_state.preset_id)),
                    down: Arc::new(Mutex::new(saved_state.down)),
                    downs_togo: Arc::new(Mutex::new(saved_state.downs_togo)),
                    countdown_text: Arc::new(Mutex::new(saved_state.countdown_text)),
                    show_countdown: Arc::new(Mutex::new(saved_state.show_countdown)),
                    show_downs: Arc::new(Mutex::new(saved_state.show_downs)),
                    show_scoreboard: Arc::new(Mutex::new(saved_state.show_scoreboard)),
                });
            } else {
                return Err(anyhow!("Failed to deserialize appstate.json"));
            }
        } else {
            return Err(anyhow!("Failed to open appstate.json"));
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AppStateSerde {
    home_points: u32,
    home_name: String,
    away_points: u32,
    away_name: String,
    quarter: u8,
    preset_id: String,
    down: u8,
    downs_togo: u8,
    countdown_text: String,
    show_countdown: bool,
    show_downs: bool,
    show_scoreboard: bool,
    show_sponsors: bool,
    ocr_api: bool,
    game_clock: usize,
    countdown_clock: usize,
}

impl AppStateSerde {
    async fn consume_app_state(state: AppState) -> AppStateSerde {
        AppStateSerde {
            home_points: *state.home_points.lock().await,
            home_name: state.home_name.lock().await.clone(),
            away_points: *state.away_points.lock().await,
            away_name: state.away_name.lock().await.clone(),
            quarter: *state.quarter.lock().await,
            preset_id: state.preset_id.lock().await.clone(),
            down: *state.down.lock().await,
            downs_togo: *state.downs_togo.lock().await,
            countdown_text: state.countdown_text.lock().await.clone(),
            show_countdown: *state.show_countdown.lock().await,
            show_downs: *state.show_downs.lock().await,
            show_scoreboard: *state.show_scoreboard.lock().await,
            show_sponsors: *SHOW_SPONSORS.lock().await,
            ocr_api: *OCR_API.lock().await,
            game_clock: *GAME_CLOCK.lock().await,
            countdown_clock: *COUNTDOWN_CLOCK.lock().await,
        }
    }
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
        let mut f = File::create("secret.key")
            .await
            .expect("Could not create secret.key");

        let key: [u8; 32] = rand::thread_rng().gen();
        let secret = BASE64_STANDARD.encode(key);

        f.write_all(secret.as_bytes())
            .await
            .expect("Could not init secret.key");
    }

    if let Err(_) = File::open("config.json").await {
        printlg!("Initializing config.json");
        let mut f = File::create("config.json")
            .await
            .expect("Cannot create config.json");

        let default_config = Config {
            secure_auth_cookie: true,
            sponsor_wait_time: 5,
            countdown_opacity: 0.5,
        };

        f.write_all(
            serde_json::to_string_pretty(&default_config)
                .expect("Could not serialize default config")
                .as_bytes(),
        )
        .await
        .expect("Could not initialize config.json")
    }

    create_dir_all(format!("./sponsors"))
        .await
        .expect("Could not create sponsors directory");

    create_dir_all(format!("./team-presets"))
        .await
        .expect("Could not create sponsors directory");

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
        .route("/popup/:t", post(popup_handler))
        .route("/reset", post(reset_handler))
        .route("/restart", post(restart_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/update", post(update_handler))
        .layer(middleware::from_fn(auth_session_layer));

    let app = Router::new()
        .route("/", head(ping_handler))
        .route("/overlay", get(overlay_handler))
        .route("/styles.css", get(css_handler))
        .route("/htmx.js", get(htmx_js_handler))
        .route("/app.js", get(app_js_handler))
        .route("/ws.js", get(ws_js_handler))
        .route("/favicon.png", get(favicon_handler))
        .route("/spinner.svg", get(spinner_handler))
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

async fn index_handler() -> impl IntoResponse {
    Html::from(include_str!("./html/index.html"))
}

async fn teaminfo_handler() -> impl IntoResponse {
    Html::from(include_str!("./html/teaminfo.html"))
}

async fn overlay_handler() -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("./html/overlay.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login/create"))
            .body(String::new())
            .unwrap();
    }
}

async fn settings_handler() -> impl IntoResponse {
    Html::from(include_str!("./html/settings.html"))
}

async fn css_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/css")
        .body(String::from(include_str!("./html/styles.css")))
        .unwrap()
}

async fn not_found_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(String::from(include_str!("./html/status_codes/404.html")))
        .unwrap()
}

// endregion: basic pages
// region: page websockets

async fn dashboard_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(
            WEBSOCKET_UPDATE_MILLIS + thread_rng().gen_range(5..=20),
        ));
        loop {
            interval.tick().await;

            let game_clock = GAME_CLOCK.lock().await;
            let countdown_clock = COUNTDOWN_CLOCK.lock().await;

            let message = format!(
                "
            <div id=\"home-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"away-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"clock-counter-minutes\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"clock-counter-seconds\" hx-swap-oob=\"innerHTML\">{:02}</div>
            <div id=\"countdown-counter-minutes\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"countdown-counter-seconds\" hx-swap-oob=\"innerHTML\">{:02}</div>
            <div id=\"downs-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"togo-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"quarter-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"uptime-value\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"backend-css-index\" hx-swap-oob=\"innerHTML\">{}</div>
            ",
                *state.home_points.lock().await,
                *state.away_points.lock().await,
                *game_clock / 1000 / 60,
                *game_clock / 1000 % 60,
                *countdown_clock / 60,
                *countdown_clock % 60,
                match *state.down.lock().await {
                    1 => "1st",
                    2 => "2nd",
                    3 => "3rd",
                    4 => "4th",
                    _ => "",
                },
                {
                    let downs_togo = state.downs_togo.lock().await;

                    if *downs_togo == 0 {
                        String::from("Hidden")
                    } else if *downs_togo == 101 {
                        String::from("Goal")
                    } else {
                        downs_togo.to_string()
                    }
                },
                match *state.quarter.lock().await {
                    1 => "1st",
                    2 => "2nd",
                    3 => "3rd",
                    4 => "4th",
                    _ => "OT",
                },
                {
                    let uptime = UPTIME_SECS.lock().await;

                    format!(
                        "{:02}:{:02}:{:02}",
                        *uptime / 3600,
                        (*uptime % 3600) / 60,
                        *uptime % 60
                    )
                },
                format!(
                    "
            <style>
                {}
            </style>",
                    if !*state.show_downs.lock().await {
                        ".downs { 
                        display: none; 
                    }"
                    } else {
                        ""
                    },
                )
            );

            if socket.send(Message::Text(message)).await.is_err() {
                return;
            }
        }
    })
}

async fn overlay_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(WEBSOCKET_UPDATE_MILLIS - thread_rng().gen_range(5..=20)));

        let countdown_opacity = COUNTDOWN_OPACITY.lock().await.clone();

        loop {
            interval.tick().await;

            let game_clock = GAME_CLOCK.lock().await;

            let message = format!(
            "
            <div id=\"ol-score\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"ol-time\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"ol-down\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"countdown-box\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"sponsor-roll\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"backend-css-overlay\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"ol-popupContainer\" hx-swap-oob=\"innerHTML\">{}</div>
            ",
            format!("{}-{}", *state.home_points.lock().await, *state.away_points.lock().await),
            {
                let quarter_display = match *state.quarter.lock().await {
                    1 => "1st",
                    2 => "2nd",
                    3 => "3rd",
                    4 => "4th",
                    _ => "OT",
                };
                if *game_clock >= 1000 * 60 {
                    format!(
                        "{}:{:02} - {}",
                        *game_clock / 1000 / 60,
                        *game_clock / 1000 % 60,
                        quarter_display
                    )
                } else {
                    format!(
                        "{}:{:02}:{:02} - {}",
                        *game_clock / 1000 / 60,
                        *game_clock / 1000 % 60,
                        *game_clock / 10 % 100,
                        quarter_display
                    )
                }
            },
            {
                let down = state.down.lock().await;

                let down_display = match *down {
                    1 => String::from("1st"),
                    2 => String::from("2nd"),
                    3 => String::from("3rd"),
                    4 => String::from("4th"),
                    _ => String::new(),
                };

                drop(down);
                let downs_togo = state.downs_togo.lock().await;

                if *downs_togo != 0 {
                    format!(
                        "{} & {}",
                        down_display,
                        if *downs_togo == 101 {
                            String::from("Goal")
                        } else {
                            downs_togo.to_string()
                        }
                    )
                } else {
                    format!("{}", down_display,)
                }
            },
            {
                if *state.show_countdown.lock().await {
                    let countdown_clock = COUNTDOWN_CLOCK.lock().await;
                    format!(
                        "<div id=\"ol-countdown\" class=\"countdown-container\" style=\"opacity: {};\"><h2 class=\"countdown-title\">{}:</h2>{}:{:02}</div>",
                        countdown_opacity,
                        state.countdown_text.lock().await,
                        *countdown_clock / 60,
                        *countdown_clock % 60
                    )
                } else {
                    String::new()
                }
            },
            {
                let sponsor_tags = SPONSOR_TAGS.lock().await;
                if *SHOW_SPONSORS.lock().await && sponsor_tags.len() > 0 {
                    let sponsor_img = sponsor_tags[*SPONSOR_IDX.lock().await].clone();
                    format!(
                        "<div class=\"ol-sponsor-parent\">{}</div>",
                        sponsor_img
                    )
                } else {
                    String::new()
                }
            },
            {
                format!(
                    "
                <style>
                    {}
                    {}
                </style>",
                    if !*state.show_downs.lock().await {
                    "
                    .ol-down-box { 
                        display: none; 
                    }"
                    } else {
                        ""
                    },
                    if !*state.show_scoreboard.lock().await {
                        ".ol-parent-container { display: none; }"
                    } else {
                        ""
                    }
                )
            },
            {
                let mut html = String::new();

                let popups_home = POPUPS_HOME.lock().await;
                let mut h_vec = Vec::new();

                for i in 0..popups_home.len() {
                    h_vec.push(format!("<span>{}</span>", popups_home[i].0));
                }

                if h_vec.len() > 0 {
                    html += &format!("<div class=\"ol-home-popup\">{}</div>", h_vec.join("<br>"));
                }

                drop(h_vec);

                let popups_away = POPUPS_AWAY.lock().await;
                let mut a_vec = Vec::new();

                for i in 0..popups_away.len() {
                    a_vec.push(format!("<span>{}</span>", popups_away[i].0));
                }

                if a_vec.len() > 0 {
                    html += &format!("<div class=\"ol-away-popup\">{}</div>", a_vec.join("<br>"));
                }

                drop(a_vec);

                html
            }
            );

            if socket.send(Message::Text(message)).await.is_err() {
                return;
            }
        }
    })
}

// endregion: page websockets
// region: js routing

async fn htmx_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("./html/js/htmx.js")))
        .unwrap()
}

async fn ws_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("./html/js/ws.js")))
        .unwrap()
}

async fn app_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("./html/js/app.js")))
        .unwrap()
}

// endregion: js routing
// region: image routing

async fn favicon_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static("image/x-icon"))
        .body(Body::from(
            include_bytes!("./html/img/favicon.png").to_vec(),
        ))
        .unwrap()
}

async fn spinner_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static("image/svg+xml"))
        .body(String::from(include_str!("./html/img/spinner.svg")))
        .unwrap()
}

// endregion: image routing
// region: login

async fn create_login_page_handler() -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login"))
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("./html/create_login.html")))
            .unwrap();
    }
}

#[derive(Serialize, Deserialize)]
struct CreateLogin {
    username: String,
    password: String,
    confirm_password: String,
}

#[derive(Serialize, Deserialize)]
struct Login {
    username: String,
    password: String,
    api_key: String,
}

#[derive(Serialize, Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct Claims {
    sub: String,
    un: String,
    exp: usize,
}

#[derive(Serialize, Deserialize)]
struct SessionClaims {
    sub: String,
    exp: usize,
}

async fn create_login_handler(Form(data): Form<CreateLogin>) -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    } else if !data.username.contains(" ")
        && !data.username.is_empty()
        && !data.password.contains(" ")
        && !data.password.is_empty()
    {
        if data.password == data.confirm_password {
            let password_hash = tokio::task::spawn_blocking(move || {
                let salt = SaltString::generate(&mut OsRng);
                let argon2 = Argon2::default();
                argon2
                    .hash_password(data.password.as_bytes(), &salt)
                    .unwrap()
                    .to_string()
            })
            .await
            .unwrap();

            let mut f = File::create("login.json")
                .await
                .expect("Could not create login.json");

            let new_login = Login {
                username: data.username,
                password: password_hash,
                api_key: key_create(32),
            };

            f.write_all(serde_json::to_string(&new_login).unwrap().as_bytes())
                .await
                .expect("Could not init login.json");

            return Response::builder()
                .status(StatusCode::SEE_OTHER)
                .header(SET_COOKIE, auth_cookie_builder(new_login.username).await)
                .header(
                    HeaderName::from_static("hx-redirect"),
                    HeaderValue::from_static("/"),
                )
                .body(String::new())
                .unwrap();
        } else {
            return Response::builder()
                .status(StatusCode::OK)
                .body(String::from(
                    "<div class=\"login-error\"><p>Passwords do not match</p></div>",
                ))
                .unwrap();
        }
    } else {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(
                "<div class=\"login-error\"><p>Username and password cannot be empty or contain spaces</p></div>",
            ))
            .unwrap();
    }
}

async fn login_page_handler() -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("./html/login.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login/create"))
            .body(String::new())
            .unwrap();
    }
}

async fn login_handler(Form(data): Form<LoginForm>) -> impl IntoResponse {
    if let Ok(f) = File::open("login.json").await {
        if !data.username.contains(" ")
            && !data.username.is_empty()
            && !data.password.contains(" ")
            && !data.password.is_empty()
        {
            let mut buf = String::new();
            let mut buf_reader = BufReader::new(f);

            buf_reader
                .read_to_string(&mut buf)
                .await
                .expect("Could not read login.json");

            let hashed_login: Login =
                serde_json::from_str(&buf).expect("Could not deserialize login.json");

            if data.username == hashed_login.username {
                if tokio::task::spawn_blocking(move || {
                    Argon2::default()
                        .verify_password(
                            data.password.as_bytes(),
                            &PasswordHash::new(&hashed_login.password)
                                .expect("Could not parse password hash"),
                        )
                        .is_ok()
                })
                .await
                .expect("Could not verify hash")
                {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .header(SET_COOKIE, auth_cookie_builder(data.username).await)
                        .header(
                            HeaderName::from_static("hx-redirect"),
                            HeaderValue::from_static("/"),
                        )
                        .body(String::new())
                        .unwrap();
                } else {
                    return Response::builder()
                        .status(StatusCode::OK)
                        .body(String::from(
                            "<div class=\"login-error\"><p>Invalid login</p></div>",
                        ))
                        .unwrap();
                }
            } else {
                return Response::builder()
                    .status(StatusCode::OK)
                    .body(String::from(
                        "<div class=\"login-error\"><p>Invalid login</p></div>",
                    ))
                    .unwrap();
            }
        } else {
            return Response::builder()
                .status(StatusCode::OK)
                .body(String::from(
                    "<div class=\"login-error\"><p>Username and password cannot be empty or contain spaces</p></div>",
                ))
                .unwrap();
        }
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn auth_cookie_builder(username: String) -> String {
    if let Ok(secret) = tokio::fs::read_to_string("./secret.key").await {
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            un: username,
            exp: (std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
                + std::time::Duration::from_secs(60 * 60 * 24 * 7).as_secs())
                as usize,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("Could not create auth token!");

        let mut config_str = String::new();

        let config_f = File::open("config.json")
            .await
            .expect("Could not open config.json");
        let mut buf_reader = BufReader::new(config_f);
        buf_reader
            .read_to_string(&mut config_str)
            .await
            .expect("Could not read config.json");

        let config: Config =
            serde_json::from_str(&config_str).expect("Could not deserialize config.json");

        let cookie = Cookie::build(("AuthToken", token))
            .path("/")
            .secure(config.secure_auth_cookie)
            .http_only(true)
            .max_age(cookie::time::Duration::days(7))
            .same_site(SameSite::Strict);

        cookie.to_string()
    } else {
        panic!("Could not read secret.key!");
    }
}

async fn session_cookie_builder() -> String {
    if let Ok(secret) = tokio::fs::read_to_string("./secret.key").await {
        let claims = SessionClaims {
            sub: Uuid::new_v4().to_string(),
            exp: (std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
                + std::time::Duration::from_secs(60 * 60 * 24 * 2).as_secs())
                as usize,
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .expect("Could not create auth token!");

        let mut config_str = String::new();

        let config_f = File::open("config.json")
            .await
            .expect("Could not open config.json");
        let mut buf_reader = BufReader::new(config_f);
        buf_reader
            .read_to_string(&mut config_str)
            .await
            .expect("Could not read config.json");

        let config: Config =
            serde_json::from_str(&config_str).expect("Could not deserialize config.json");

        let cookie = Cookie::build(("SessionToken", token))
            .path("/")
            .secure(config.secure_auth_cookie)
            .http_only(true)
            .same_site(SameSite::Strict);

        cookie.to_string()
    } else {
        panic!("Could not read secret.key!");
    }
}

async fn verify_auth(jar: CookieJar) -> bool {
    if let Some(auth_token) = jar.get("AuthToken") {
        let validation = Validation::new(jsonwebtoken::Algorithm::HS256);

        let mut secret = String::new();

        let secret_f = File::open("secret.key")
            .await
            .expect("Could not open secret.key");
        let mut buf_reader = BufReader::new(secret_f);
        buf_reader
            .read_to_string(&mut secret)
            .await
            .expect("Could not read secret.key");

        if let Ok(_) = decode::<Claims>(
            &auth_token.value(),
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        ) {
            return true;
        } else {
            return false;
        }
    } else {
        return false;
    }
}

async fn verify_session(jar: CookieJar) -> bool {
    if let Some(auth_token) = jar.get("SessionToken") {
        let validation = Validation::new(jsonwebtoken::Algorithm::HS256);

        let mut secret = String::new();

        let secret_f = File::open("secret.key")
            .await
            .expect("Could not open secret.key");
        let mut buf_reader = BufReader::new(secret_f);
        buf_reader
            .read_to_string(&mut secret)
            .await
            .expect("Could not read secret.key");

        if let Ok(_) = decode::<SessionClaims>(
            &auth_token.value(),
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        ) {
            return true;
        } else {
            return false;
        }
    } else {
        return false;
    }
}

async fn ping_handler() -> impl IntoResponse {
    return StatusCode::OK;
}

// endregion: login
// region: team routing

async fn home_points_update_handler(
    Path(a): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut home_points = state.home_points.lock().await;

    if *home_points as i32 + a >= 0 {
        *home_points = (*home_points as i32 + a) as u32;
    }

    printlg!("UPDATE home_points: {}", *home_points);

    return StatusCode::OK;
}

async fn home_points_set_handler(
    Path(a): Path<u32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut home_points = state.home_points.lock().await;
    *home_points = a;

    printlg!("SET home_points: {}", *home_points);

    return StatusCode::OK;
}

async fn away_points_update_handler(
    Path(a): Path<i32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut away_points = state.away_points.lock().await;

    if *away_points as i32 + a >= 0 {
        *away_points = (*away_points as i32 + a) as u32;
    }

    printlg!("UPDATE away_points: {}", *away_points);

    return StatusCode::OK;
}

async fn away_points_set_handler(
    Path(a): Path<u32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut away_points = state.away_points.lock().await;
    *away_points = a;

    printlg!("SET home_points: {}", *away_points);

    return StatusCode::OK;
}

async fn home_points_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.home_points.lock().await.to_string()
}

async fn away_points_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.away_points.lock().await.to_string()
}

// endregion: team routing
// region: time

async fn uptime_ticker() {
    let start_time = Instant::now();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut uptime_secs = UPTIME_SECS.lock().await;

        *uptime_secs = (Instant::now() - start_time).as_secs() as usize;
    }
}

async fn game_clock_ticker() {
    loop {
        let call_time = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let mut game_clock = GAME_CLOCK.lock().await;
        let mut game_clock_start = GAME_CLOCK_START.lock().await;

        if *game_clock_start && !*OCR_API.lock().await {
            let time_diff = -1 * (Instant::now() - call_time).as_millis() as isize;
            if *game_clock as isize + time_diff >= 0 {
                *game_clock = (*game_clock as isize + time_diff) as usize;
            } else {
                *game_clock_start = false;
            }
        }
    }
}

async fn game_clock_ctl_handler(Path(a): Path<String>) -> impl IntoResponse {
    let mut game_clock_start = GAME_CLOCK_START.lock().await;

    if a == "start" {
        *game_clock_start = true;
    } else if a == "stop" {
        *game_clock_start = false;
    }

    printlg!("UPDATE game_clock_start: {}", *game_clock_start);

    return StatusCode::OK;
}

async fn game_clock_set_handler(Path(mins): Path<usize>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    *game_clock = mins * 60 * 1000;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

async fn game_clock_update_handler(Path((mins, secs)): Path<(isize, isize)>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    let time_diff = mins * 60 * 1000 + secs * 1000;

    if *game_clock as isize + time_diff >= 0 {
        *game_clock = (*game_clock as isize + time_diff) as usize;
    }

    printlg!("UPDATE game_clock: {}", game_clock);

    return StatusCode::OK;
}

async fn game_clock_display_handler(Path(o): Path<String>) -> impl IntoResponse {
    let game_clock = GAME_CLOCK.lock().await;
    let mut time_display = String::new();

    if o == "minutes" {
        time_display = (*game_clock / 1000 / 60).to_string();
    } else if o == "seconds" {
        time_display = (*game_clock / 1000 % 60).to_string();
    } else if o == "both" {
        if *game_clock > 1000 * 60 {
            time_display = format!("{}:{:02}", *game_clock / 1000 / 60, *game_clock / 1000 % 60);
        } else {
            time_display = format!(
                "{}:{:02}:{:02}",
                *game_clock / 1000 / 60,
                *game_clock / 1000 % 60,
                *game_clock / 10 % 100
            );
        }
    }

    time_display
}

async fn countdown_clock_ticker() {
    loop {
        let call_time = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
        let mut countdown_clock_start = COUNTDOWN_CLOCK_START.lock().await;

        if *countdown_clock_start {
            let time_diff = -1 * (Instant::now() - call_time).as_secs() as isize;
            if *countdown_clock as isize + time_diff >= 0 {
                *countdown_clock = (*countdown_clock as isize + time_diff) as usize;
            } else {
                *countdown_clock_start = false;
            }
        }
    }
}

async fn countdown_clock_ctl_handler(Path(a): Path<String>) -> impl IntoResponse {
    let mut countdown_clock_start = COUNTDOWN_CLOCK_START.lock().await;

    if a == "start" {
        *countdown_clock_start = true;
    } else if a == "stop" {
        *countdown_clock_start = false;
    }

    printlg!("UPDATE countdown_clock_start: {}", *countdown_clock_start);

    return StatusCode::OK;
}

async fn countdown_clock_set_handler(Path(mins): Path<usize>) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    *countdown_clock = mins * 60;

    printlg!("SET countdown_clock: {}", *countdown_clock);

    return StatusCode::OK;
}

async fn countdown_clock_update_handler(
    Path((mins, secs)): Path<(isize, isize)>,
) -> impl IntoResponse {
    let mut coundown_clock = COUNTDOWN_CLOCK.lock().await;
    let time_diff = mins * 60 + secs;

    if *coundown_clock as isize + time_diff >= 0 {
        *coundown_clock = (*coundown_clock as isize + time_diff) as usize;
    }

    printlg!("UPDATE countdown_clock: {}", *coundown_clock);

    return StatusCode::OK;
}

async fn countdown_clock_display_handler(Path(o): Path<String>) -> impl IntoResponse {
    let countdown_clock = COUNTDOWN_CLOCK.lock().await;
    let mut time_display = String::new();

    if o == "minutes" {
        time_display = (*countdown_clock / 60).to_string();
    } else if o == "seconds" {
        time_display = (*countdown_clock % 60).to_string();
    } else if o == "both" {
        time_display = format!("{}:{:02}", *countdown_clock / 60, *countdown_clock % 60);
    }

    time_display
}

#[derive(Deserialize)]
struct TextPayload {
    text: String,
}

async fn countdown_text_set_handler(
    State(state): State<AppState>,
    Form(payload): Form<TextPayload>,
) -> impl IntoResponse {
    let mut countdown_text = state.countdown_text.lock().await;
    *countdown_text = payload.text;

    printlg!("SET countdown_text: {}", countdown_text);

    return StatusCode::OK;
}

// endregion: time
// region: quarters

async fn quarter_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    let quarter = state.quarter.lock().await;

    let event_body = match *quarter {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        4 => "4th",
        _ => "OT",
    };

    event_body
}

async fn quarter_set_handler(
    Path(q): Path<u8>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut quarter = state.quarter.lock().await;
    *quarter = q;

    printlg!("SET quarter: {}", *quarter);

    return StatusCode::OK;
}

async fn quarter_update_handler(
    State(state): State<AppState>,
    Path(a): Path<i8>,
) -> impl IntoResponse {
    let mut quarter = state.quarter.lock().await;

    if *quarter as i8 + a >= 1 && *quarter as i8 + a <= 5 {
        *quarter = (*quarter as i8 + a) as u8;
    }

    printlg!("UPDATE quarter: {}", *quarter);

    return StatusCode::OK;
}

// endregion: quarters
// region: teaminfo

#[derive(Serialize, Deserialize)]
struct Teaminfo {
    home_name: String,
    home_color: String,
    away_name: String,
    away_color: String,
}
impl Teaminfo {
    fn new() -> Teaminfo {
        Teaminfo {
            home_name: String::new(),
            home_color: String::new(),
            away_name: String::new(),
            away_color: String::new(),
        }
    }
}

async fn teaminfo_preset_create_handler(mut form: Multipart) -> impl IntoResponse {
    let mut teaminfo = Teaminfo::new();
    let id = id_create(12);

    create_dir_all(format!("team-presets/{}", id))
        .await
        .expect("Could not create team preset directory");

    while let Some(field) = form
        .next_field()
        .await
        .expect("Could not get next field of preset create multipart")
    {
        match field.name().unwrap() {
            "home_name" => {
                teaminfo.home_name = field.text().await.unwrap();
            }
            "home_img" => {
                let mut f = File::create(format!(
                    "team-presets/{}/home.{}",
                    id,
                    field
                        .file_name()
                        .unwrap()
                        .to_string()
                        .split(".")
                        .collect::<Vec<&str>>()[1]
                ))
                .await
                .expect("Could not create home img");

                f.write_all(field.bytes().await.unwrap().as_ref())
                    .await
                    .expect("Could not write to home img");
            }
            "home_color" => {
                teaminfo.home_color = field.text().await.unwrap();
            }
            "away_name" => {
                teaminfo.away_name = field.text().await.unwrap();
            }
            "away_img" => {
                let mut f = File::create(format!(
                    "team-presets/{}/away.{}",
                    id,
                    field
                        .file_name()
                        .unwrap()
                        .to_string()
                        .split(".")
                        .collect::<Vec<&str>>()[1]
                ))
                .await
                .expect("Could not create away img");

                f.write_all(field.bytes().await.unwrap().as_ref())
                    .await
                    .expect("Could not write to away img");
            }
            "away_color" => {
                teaminfo.away_color = field.text().await.unwrap();
            }
            _ => {}
        }
    }

    let write_json = serde_json::to_string_pretty(&teaminfo).expect("Could not serialize teaminfo");

    let mut f = File::create(format!("team-presets/{}/teams.json", id))
        .await
        .expect("Could not create teams.json");
    f.write_all(write_json.as_bytes())
        .await
        .expect("Could not write to teams.json");

    printlg!(
        "CREATE teaminfo_preset: {} vs {} (id: {})",
        teaminfo.home_name,
        teaminfo.away_name,
        id
    );

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-selector"),
        )
        .body(String::new())
        .unwrap();
}

async fn teaminfo_preset_selector_handler() -> impl IntoResponse {
    let mut html = String::new();
    let mut a = read_dir("./team-presets").await.unwrap();

    while let Ok(Some(d)) = a.next_entry().await {
        if d.file_type()
            .await
            .expect("Could not get preset file type")
            .is_dir()
        {
            let mut home_img_path = PathBuf::new();
            let mut away_img_path = PathBuf::new();

            let mut home_tag_type = "";
            let mut away_tag_type = "";

            let mut teaminfo = Teaminfo::new();

            let mut b = read_dir(d.path()).await.unwrap();

            while let Ok(Some(d0)) = b.next_entry().await {
                let file_name = d0.file_name().to_string_lossy().to_string();

                if file_name.starts_with("home.") {
                    home_img_path = d0.path();

                    home_tag_type = match file_name.split(".").collect::<Vec<&str>>()[1] {
                        "png" => "png",
                        "jpg" => "jpeg",
                        "jpeg" => "jpeg",
                        _ => "",
                    }
                } else if file_name.starts_with("away.") {
                    away_img_path = d0.path();

                    away_tag_type = match file_name.split(".").collect::<Vec<&str>>()[1] {
                        "png" => "png",
                        "jpg" => "jpeg",
                        "jpeg" => "jpeg",
                        _ => "",
                    }
                } else if file_name == "teams.json" {
                    let f = File::open(d0.path())
                        .await
                        .expect("Could not open teams.json");
                    let mut buf_reader = BufReader::new(f);

                    let mut temp_str = String::new();

                    buf_reader
                        .read_to_string(&mut temp_str)
                        .await
                        .expect("Could not read teams.json");

                    teaminfo =
                        serde_json::from_str(&temp_str).expect("Could not deserialize teams.json");
                }
            }
            let home_img_bytes = tokio::fs::read(home_img_path)
                .await
                .expect("Could not read home img");
            let away_img_bytes = tokio::fs::read(away_img_path)
                .await
                .expect("Could not read away img");

            let id = d.file_name().to_string_lossy().to_string();

            html += &format!(
            "<div class=\"match-selector\">
                <img class=\"home-logo\" src=\"data:image/{};base64,{}\" alt=\"home-img\" height=\"30px\" width=\"auto\" style=\"border-color: {}; border-style: solid; border-radius: 3px; border-width: 2px\">
                <p class=\"teampreset-title\">{} vs {}</p>
                <img class=\"away-logo\" src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"30px\" width=\"auto\" style=\"border-color: {}; border-style: solid; border-radius: 3px; border-width: 2px;\">
                <button class=\"select-button\" hx-post=\"/teaminfo/select/{}\" hx-swap=\"none\">Select</button>
                <button class=\"remove-button\" hx-post=\"/teaminfo/remove/{}\" hx-swap=\"none\">Remove</button>
            </div>",
                home_tag_type,
                BASE64_STANDARD.encode(home_img_bytes),
                teaminfo.home_color,
                teaminfo.home_name,
                teaminfo.away_name,
                away_tag_type,
                BASE64_STANDARD.encode(away_img_bytes),
                teaminfo.away_color,
                id,
                id
            );
        }
    }

    return Html::from(html);
}

async fn teaminfo_preset_select_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut dir = read_dir("./team-presets").await.unwrap();

    while let Ok(Some(a)) = dir.next_entry().await {
        if a.file_type()
            .await
            .expect("Could not get file type of dir entry")
            .is_dir()
        {
            if a.file_name().to_string_lossy().to_string() == id {
                *state.preset_id.lock().await = id.clone();

                let mut a_json = String::new();

                let a_json_f = File::open(format!(
                    "{}/teams.json",
                    a.path().to_string_lossy().to_string()
                ))
                .await
                .expect("Could not open preset file");
                let mut buf_reader = BufReader::new(a_json_f);

                buf_reader
                    .read_to_string(&mut a_json)
                    .await
                    .expect("Could not read preset file");

                let team_info: Teaminfo =
                    serde_json::from_str(&a_json).expect("Could not deserialize preset file");

                printlg!(
                    "SELECT teaminfo_preset: {} vs {} (id: {})",
                    team_info.home_name,
                    team_info.away_name,
                    id
                );

                *state.home_name.lock().await = team_info.home_name;
                *state.away_name.lock().await = team_info.away_name;

                break;
            }
        }
    }

    return StatusCode::OK;
}

async fn teaminfo_preset_remove_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Ok(_) = remove_dir_all(format!("./team-presets/{}", id)).await {
        *state.preset_id.lock().await = String::new();
        printlg!("REMOVE teaminfo_preset: {}", id);
    }

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-selector"),
        )
        .body(String::new())
        .unwrap();
}

async fn team_name_display_handler(
    Path(t): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if t == "home" {
        return Html::from(state.home_name.lock().await.clone());
    } else if t == "away" {
        return Html::from(state.away_name.lock().await.clone());
    } else {
        return Html::from(String::new());
    }
}

async fn teaminfo_button_css_handler(State(state): State<AppState>) -> impl IntoResponse {
    let preset_id = state.preset_id.lock().await;
    if preset_id.is_empty() {
        return Html::from(String::new());
    } else {
        if let Ok(teaminfo) = serde_json::from_str::<Teaminfo>(
            &tokio::fs::read_to_string(format!("./team-presets/{}/teams.json", *preset_id))
                .await
                .unwrap(),
        ) {
            let home_rgb = hex_to_rgb(&teaminfo.home_color);
            let home_rgb_complimentary =
                ((255 - home_rgb.0), (255 - home_rgb.1), (255 - home_rgb.2));
            let home_rgb_grayscale_nudge = (((255.0 - home_rgb_complimentary.0 as f32)
                * (home_rgb_complimentary.0 as f32 / 255.0)
                + (255.0 - home_rgb_complimentary.1 as f32)
                    * (home_rgb_complimentary.1 as f32 / 255.0)
                + (255.0 - home_rgb_complimentary.2 as f32)
                    * (home_rgb_complimentary.2 as f32 / 255.0))
                / 3.0) as u8;
            let home_rgb_grayscale_value = ((home_rgb_complimentary.0 as u32
                + home_rgb_complimentary.1 as u32
                + home_rgb_complimentary.2 as u32)
                / 3) as u8;
            let home_text_color = rgb_to_hex(&(
                home_rgb_grayscale_value + home_rgb_grayscale_nudge,
                home_rgb_grayscale_value + home_rgb_grayscale_nudge,
                home_rgb_grayscale_value + home_rgb_grayscale_nudge,
            ));

            let away_rgb = hex_to_rgb(&teaminfo.away_color);
            let away_rgb_complimentary =
                ((255 - away_rgb.0), (255 - away_rgb.1), (255 - away_rgb.2));
            let away_rgb_grayscale_nudge = (((255.0 - away_rgb_complimentary.0 as f32)
                * (away_rgb_complimentary.0 as f32 / 255.0)
                + (255.0 - away_rgb_complimentary.1 as f32)
                    * (away_rgb_complimentary.1 as f32 / 255.0)
                + (255.0 - away_rgb_complimentary.2 as f32)
                    * (away_rgb_complimentary.2 as f32 / 255.0))
                / 3.0) as u8;
            let away_rgb_grayscale_value = ((away_rgb_complimentary.0 as u32
                + away_rgb_complimentary.1 as u32
                + away_rgb_complimentary.2 as u32)
                / 3) as u8;
            let away_text_color = rgb_to_hex(&(
                away_rgb_grayscale_value + away_rgb_grayscale_nudge,
                away_rgb_grayscale_value + away_rgb_grayscale_nudge,
                away_rgb_grayscale_value + away_rgb_grayscale_nudge,
            ));

            return Html::from(format!(
                "
            <style>
                .button-decrement-home {{
                    background-color: {};
                    color: {};
                }}
                .button-increment-home {{
                    background-color: {};
                    color: {};
                }}
                .button-preset-score-home {{
                    background-color: {};
                    color: {};
                }}
                .button-decrement-away {{
                    background-color: {};
                    color: {};
                }}
                .button-increment-away {{
                    background-color: {};
                    color: {};
                }}
                .button-preset-score-away {{
                    background-color: {};
                    color: {};
                }}
            </style>
            ",
                teaminfo.home_color,
                home_text_color,
                teaminfo.home_color,
                home_text_color,
                teaminfo.home_color,
                home_text_color,
                teaminfo.away_color,
                away_text_color,
                teaminfo.away_color,
                away_text_color,
                teaminfo.away_color,
                away_text_color,
            ));
        } else {
            return Html::from(String::new());
        }
    }
}

// endregion: teaminfo
// region: sponsors

async fn upload_sponsors_handler(mut form: Multipart) -> impl IntoResponse {
    create_dir_all(format!("./sponsors"))
        .await
        .expect("Could not create sponsors directory");

    while let Some(field) = form
        .next_field()
        .await
        .expect("Could not get next field of sponsor multipart")
    {
        let id = id_create(12);
        let mut f = File::create(format!(
            "./sponsors/{}.{}",
            id,
            field.file_name().unwrap().split(".").collect::<Vec<&str>>()[1]
        ))
        .await
        .expect("Could not create sponsor file");

        f.write_all(field.bytes().await.unwrap().as_ref())
            .await
            .expect("Could not write to sponsor file");

        println!("ADD sponsor: {}", id);
    }

    load_sponsors().await;

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-sponsor"),
        )
        .body(String::new())
        .unwrap();
}

async fn sponsors_management_handler() -> impl IntoResponse {
    let mut d = read_dir("./sponsors").await.unwrap();
    let mut html = String::new();

    while let Ok(Some(a)) = d.next_entry().await {
        let fname = a.file_name().to_string_lossy().to_string();
        let fname_vec = fname.split(".").collect::<Vec<&str>>();

        let mime = match fname_vec[1] {
            "png" => "png",
            "jpg" => "jpeg",
            "jpeg" => "jpeg",
            _ => "",
        };

        let f_bytes = tokio::fs::read(a.path())
            .await
            .expect("Could not read sponsor image");

        html += &format!(
            "<div class=\"sponsor-wrapper\">
                <img src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"30px\" width=\"auto\">
                <button class=\"remove-button\" hx-post=\"/sponsors/remove/{}\" hx-swap=\"none\">Remove</button>
            </div>",
            mime,
            BASE64_STANDARD.encode(f_bytes),
            fname_vec[0]
        );
    }

    return Html::from(html);
}

async fn sponsor_remove_handler(Path(id): Path<String>) -> impl IntoResponse {
    let mut d = read_dir("./sponsors").await.unwrap();
    let mut p = PathBuf::new();

    while let Ok(Some(a)) = d.next_entry().await {
        if a.file_name()
            .to_string_lossy()
            .to_string()
            .split(".")
            .collect::<Vec<&str>>()[0]
            == id
        {
            p = a.path();
            break;
        }
    }

    remove_file(p).await.expect("Could not remove sponsor file");

    printlg!("REMOVE sponsor: {}", id);

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-sponsor"),
        )
        .body(String::new())
        .unwrap();
}

async fn load_sponsors() {
    create_dir_all(format!("./sponsors"))
        .await
        .expect("Could not create sponsors directory");

    let mut d = read_dir("./sponsors")
        .await
        .expect("Could not read sponsors dir");

    while let Ok(Some(f)) = d.next_entry().await {
        let fname = f.file_name().to_string_lossy().to_string();

        let mime_type = match fname.split(".").collect::<Vec<&str>>()[1] {
            "png" => "png",
            "jpg" => "jpeg",
            "jpeg" => "jpeg",
            _ => "",
        };

        let f_bytes = tokio::fs::read(f.path())
            .await
            .expect("Could not read sponsor image");

        *SPONSOR_IDX.lock().await = 0;
        SPONSOR_TAGS.lock().await.push(format!(
            "<img class=\"ol-sponsor-img\" src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"auto\">",
            mime_type,
            BASE64_STANDARD.encode(f_bytes),
        ))
    }
}

async fn sponsor_ticker() {
    let cfg = tokio::fs::read_to_string("./config.json")
        .await
        .expect("Could not read config json");
    let cfg_json: Config = serde_json::from_str(&cfg).expect("Could not deserialize config json");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(cfg_json.sponsor_wait_time)).await;
        let mut sponsor_idx = SPONSOR_IDX.lock().await;
        let show_sponsors = SHOW_SPONSORS.lock().await;

        if *show_sponsors {
            if *sponsor_idx < SPONSOR_TAGS.lock().await.len() - 1 {
                *sponsor_idx += 1;
            } else {
                *sponsor_idx = 0;
            }
        }
    }
}

// endregion: sponsors
// region: overlay

async fn icon_handler(Path(t): Path<String>, State(state): State<AppState>) -> impl IntoResponse {
    let mut d = read_dir(format!("./team-presets/{}", state.preset_id.lock().await))
        .await
        .expect("Could not read preset dir");

    while let Ok(Some(f)) = d.next_entry().await {
        if !f.file_type().await.unwrap().is_dir() {
            let fname = f.file_name().to_string_lossy().to_string();

            if t == "home" && fname.starts_with("home.") {
                let img_bytes = tokio::fs::read(f.path())
                    .await
                    .expect("Could not read img bytes");

                let mime_type = match fname.split(".").collect::<Vec<&str>>()[1] {
                    "png" => "png",
                    "jpg" => "jpeg",
                    "jpeg" => "jpeg",
                    _ => "",
                };

                return Html::from(format!("<img class=\"ol-team-logo\" src=\"data:image/{};base64,{}\" height=\"30px\" width=\"auto\" alt=\"home-icon\">", mime_type, BASE64_STANDARD.encode(img_bytes)));
            } else if t == "away" && fname.starts_with("away.") {
                let img_bytes = tokio::fs::read(f.path())
                    .await
                    .expect("Could not read img bytes");

                let mime_type = match fname.split(".").collect::<Vec<&str>>()[1] {
                    "png" => "png",
                    "jpg" => "jpeg",
                    "jpeg" => "jpeg",
                    _ => "",
                };

                return Html::from(format!("<img class=\"ol-team-logo\" src=\"data:image/{};base64,{}\" height=\"30px\" width=\"auto\" alt=\"away-icon\">", mime_type, BASE64_STANDARD.encode(img_bytes)));
            }
        }
    }

    return Html::from(String::new());
}

async fn overlay_team_border_css_handler(State(state): State<AppState>) -> impl IntoResponse {
    let preset_id = state.preset_id.lock().await;
    if preset_id.is_empty() {
        return Html::from(String::new());
    } else {
        if let Ok(teaminfo) = serde_json::from_str::<Teaminfo>(
            &tokio::fs::read_to_string(format!("./team-presets/{}/teams.json", *preset_id))
                .await
                .unwrap(),
        ) {
            return Html::from(format!(
                "
            <style>
                .ol-home-box, .button {{
                    border-color: {};
                    border-style: solid;
                }}
                .ol-away-box {{
                    border-color: {};
                    border-style: solid;
                }}
            </style>
            ",
                teaminfo.home_color, teaminfo.away_color
            ));
        } else {
            return Html::from(String::new());
        }
    }
}

// endregion: overlay
// region: downs

async fn downs_set_handler(State(state): State<AppState>, Path(d): Path<u8>) -> impl IntoResponse {
    if (1..=4).contains(&d) {
        let mut down = state.down.lock().await;
        *down = d;

        printlg!("SET down: {}", *down);
    }

    return StatusCode::OK;
}

async fn downs_update_handler(
    State(state): State<AppState>,
    Path(y): Path<i8>,
) -> impl IntoResponse {
    let mut down = state.down.lock().await;

    let new_val = *down as i8 + y;

    if (1..=4).contains(&new_val) {
        *down = (*down as i8 + y) as u8;
        printlg!("UPDATE down: {}", *down);
    } else if new_val < 1 {
        *down = 4;
        printlg!("UPDATE down: {}", *down);
    } else if new_val > 4 {
        *down = 1;
        printlg!("UPDATE down: {}", *down);
    }

    return StatusCode::OK;
}

async fn downs_togo_set_handler(
    State(state): State<AppState>,
    Path(y): Path<u8>,
) -> impl IntoResponse {
    if (0..=101).contains(&y) {
        let mut togo = state.downs_togo.lock().await;
        *togo = y;

        printlg!("SET togo: {}", *togo);
    }

    return StatusCode::OK;
}

async fn downs_togo_update_handler(
    State(state): State<AppState>,
    Path(y): Path<i8>,
) -> impl IntoResponse {
    let mut downs_togo = state.downs_togo.lock().await;

    let new_val = *downs_togo as i8 + y;

    if (0..=101).contains(&new_val) {
        *downs_togo = new_val as u8;

        printlg!("UPDATE togo: {}", *downs_togo);
    } else if new_val > 101 {
        *downs_togo = 0;

        printlg!("UPDATE togo: {}", *downs_togo);
    } else if new_val < 0 {
        *downs_togo = 101;

        printlg!("UPDATE togo: {}", *downs_togo);
    }

    return StatusCode::OK;
}

async fn downs_display_handler(
    State(state): State<AppState>,
    Path(t): Path<String>,
) -> impl IntoResponse {
    if t == "down" {
        let down = state.down.lock().await;

        return match *down {
            1 => String::from("1st"),
            2 => String::from("2nd"),
            3 => String::from("3rd"),
            4 => String::from("4th"),
            _ => String::new(),
        };
    } else if t == "togo" {
        let downs_togo = state.downs_togo.lock().await;

        if *downs_togo == 101 {
            return String::from("Goal");
        } else if *downs_togo == 0 {
            return String::from("Hidden");
        } else {
            return downs_togo.to_string();
        }
    } else if t == "both" {
        let down = state.down.lock().await;

        let down_display = match *down {
            1 => String::from("1st"),
            2 => String::from("2nd"),
            3 => String::from("3rd"),
            4 => String::from("4th"),
            _ => String::new(),
        };

        drop(down);
        let downs_togo = state.downs_togo.lock().await;

        if *downs_togo != 0 {
            return format!(
                "{} & {}",
                down_display,
                if *downs_togo == 101 {
                    String::from("Goal")
                } else {
                    downs_togo.to_string()
                }
            );
        } else {
            return format!("{}", down_display);
        }
    } else {
        return String::new();
    }
}

// endregion: downs
// region: visibility

async fn visibility_buttons_handler(State(state): State<AppState>) -> impl IntoResponse {
    return Html::from(format!(
        "
    <div class=\"display-buttons\">
        <button class=\"show-countdown\" hx-post=\"/visibility/toggle/countdown\">{}</button>
        <button class=\"show-downs\" hx-post=\"/visibility/toggle/downs\">{}</button>
        <button class=\"show-scoreboard\" hx-post=\"/visibility/toggle/scoreboard\">{}</button>
        <button class=\"show-sponsors\" hx-post=\"/visibility/toggle/sponsors\">{}</button>
    </div>
    ",
        if !*state.show_countdown.lock().await {
            "Show Countdown"
        } else {
            "Hide Countdown"
        },
        if !*state.show_downs.lock().await {
            "Show Downs/To Go"
        } else {
            "Hide Downs/To Go"
        },
        if !*state.show_scoreboard.lock().await {
            "Show Scoreboard"
        } else {
            "Hide Scoreboard"
        },
        if !*SHOW_SPONSORS.lock().await {
            "Show Sponsors"
        } else {
            "Hide Sponsors"
        }
    ));
}

async fn visibility_toggle_handler(
    State(state): State<AppState>,
    Path(v): Path<String>,
) -> impl IntoResponse {
    let mut modified = ("", false);

    match v.as_str() {
        "countdown" => {
            let mut countdown = state.show_countdown.lock().await;

            if *countdown {
                *countdown = false;
            } else {
                *countdown = true;
            }
            modified = ("Countdown", *countdown);

            printlg!("SET {} visibility: {}", modified.0, *countdown);
        }
        "downs" => {
            let mut downs = state.show_downs.lock().await;

            if *downs {
                *downs = false;
            } else {
                *downs = true;
            }
            modified = ("Downs/To Go", *downs);

            printlg!("SET {} visibility: {}", modified.0, *downs);
        }
        "scoreboard" => {
            let mut scoreboard = state.show_scoreboard.lock().await;

            if *scoreboard {
                *scoreboard = false;
            } else {
                *scoreboard = true;
            }
            modified = ("Scoreboard", *scoreboard);

            printlg!("SET {} visibility: {}", modified.0, *scoreboard);
        }
        "sponsors" => {
            let mut sponsors = SHOW_SPONSORS.lock().await;

            if *sponsors {
                *sponsors = false;
            } else {
                *sponsors = true;
            }
            modified = ("Sponsors", *sponsors);

            printlg!("SET {} visibility: {}", modified.0, *sponsors);
        }
        _ => {}
    }

    return Response::builder()
        .status(StatusCode::OK)
        .body(format!(
            "{} {}",
            if !modified.1 { "Show" } else { "Hide" },
            modified.0
        ))
        .unwrap();
}

// endregion: visibility
// region: api

async fn api_key_check_handler(Path(k): Path<String>) -> impl IntoResponse {
    let login: Login = serde_json::from_str(
        &tokio::fs::read_to_string("./login.json")
            .await
            .expect("Could not read login.json"),
    )
    .expect("Could not deserialize login.json");

    if k == login.api_key {
        return StatusCode::OK;
    } else {
        return StatusCode::UNAUTHORIZED;
    }
}

async fn ocr_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    if let Some(api_key) = headers.get("api-key") {
        if *OCR_API.lock().await {
            let login: Login = serde_json::from_str(
                &tokio::fs::read_to_string("./login.json")
                    .await
                    .expect("Could not read login.json"),
            )
            .expect("Could not deserialize login.json");

            if api_key.to_str().expect("Could not cast HeaderValue to str") == login.api_key {
                let ocr_data: HashMap<String, String> =
                    serde_json::from_str(&body).expect("Could not deserialize request body");

                if let Some(d) = ocr_data.get("Time") {
                    let time_vec = d.split(":").collect::<Vec<&str>>();

                    if time_vec.len() == 2 && time_vec.iter().all(|x| x.parse::<u32>().is_ok()) {
                        let time_vec: Vec<u32> =
                            time_vec.iter().map(|x| x.parse::<u32>().unwrap()).collect();

                        *GAME_CLOCK.lock().await = (time_vec[0] * 60 + time_vec[1]) as usize;
                    }
                }

                if let Some(d) = ocr_data.get("Home Score") {
                    if let Ok(s) = d.parse::<u32>() {
                        *state.home_points.lock().await = s;
                    }
                }

                if let Some(d) = ocr_data.get("Away Score") {
                    if let Ok(s) = d.parse::<u32>() {
                        *state.away_points.lock().await = s;
                    }
                }

                if let Some(d) = ocr_data.get("Period") {
                    if d.ends_with("st")
                        || d.ends_with("nd")
                        || d.ends_with("rd")
                        || d.ends_with("th")
                    {
                        *state.quarter.lock().await = d
                            .split_at(1)
                            .0
                            .parse::<u8>()
                            .expect("Could not parse quarter from ocr data");
                    } else if d.parse::<u8>().is_ok() {
                        *state.quarter.lock().await = d
                            .parse::<u8>()
                            .expect("Could not parse quarter from ocr data");
                    }
                }

                if let Some(d) = ocr_data.get("To Go") {
                    if let Ok(s) = d.parse::<u8>() {
                        *state.downs_togo.lock().await = s;
                    }
                }

                if let Some(d) = ocr_data.get("Down") {
                    if d.ends_with("st")
                        || d.ends_with("nd")
                        || d.ends_with("rd")
                        || d.ends_with("th")
                    {
                        *state.down.lock().await = d
                            .split_at(1)
                            .0
                            .parse::<u8>()
                            .expect("Could not parse quarter from ocr data");
                    } else if d.parse::<u8>().is_ok() {
                        *state.down.lock().await = d
                            .parse::<u8>()
                            .expect("Could not parse quarter from ocr data");
                    }
                }

                printlg!("OCR UPDATE: {}", body);

                return StatusCode::OK;
            } else {
                return StatusCode::UNAUTHORIZED;
            }
        } else {
            return StatusCode::CONFLICT;
        }
    } else {
        return StatusCode::UNAUTHORIZED;
    }
}

async fn ocr_api_toggle_handler() -> impl IntoResponse {
    let mut ocr_api = OCR_API.lock().await;

    if !*ocr_api {
        *ocr_api = true;
    } else {
        *ocr_api = false;
    }

    printlg!("SET ocr_api: {}", *ocr_api);

    return Response::builder()
        .status(StatusCode::OK)
        .body(format!(
            "{} OCR API",
            if !*ocr_api { "Enable" } else { "Disable" }
        ))
        .unwrap();
}

async fn ocr_api_button_handler() -> impl IntoResponse {
    return Html::from(format!(
        "<button class=\"button-settings\" hx-post=\"/ocr/api/toggle\">{} OCR API</button>",
        if !*OCR_API.lock().await {
            "Enable"
        } else {
            "Disable"
        }
    ));
}

async fn api_key_show_handler() -> impl IntoResponse {
    let login: Login = serde_json::from_str(
        &tokio::fs::read_to_string("./login.json")
            .await
            .expect("Could not read login.json"),
    )
    .expect("Could not deserialize config.json");

    let mut chars = login.api_key.chars();
    let mut hidden_key = String::new();
    while let Some(_) = chars.next() {
        hidden_key.push('*');
    }

    return Response::builder()
    .status(StatusCode::OK)
    .body(format!(
    "<div class=\"settings-box-api\">
        <h6 id=\"api-key\">{}</h6>
        <button class=\"copy-button\" onclick=\"apiCopy(this, '{}')\">Copy</button>
        <button class=\"button-settings\" hx-post=\"/api/key/reveal\" hx-target=\"#api-key\">Reveal Key</button>
        </div>",
        hidden_key, login.api_key
    ))
    .unwrap();
}

async fn api_key_reveal_handler() -> impl IntoResponse {
    let login: Login = serde_json::from_str(
        &tokio::fs::read_to_string("./login.json")
            .await
            .expect("Could not read login.json"),
    )
    .expect("Could not deserialize config.json");

    return Response::builder()
        .status(StatusCode::OK)
        .body(format!("{}", login.api_key))
        .unwrap();
}

// endregion: api
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

const REMOTE_CARGO_TOML_URL: &'static str =
    "https://github.com/AllLiver/Froggi/blob/main/Cargo.toml";

async fn update_handler() -> impl IntoResponse {
    printlg!("Checking if an update is availible...");

    if let Ok(response) = reqwest::get(REMOTE_CARGO_TOML_URL).await {
        let remote_version_str_raw = response.text().await.expect("Failed to get response text");
        let remote_version_str = remote_version_str_raw
            .split("\n")
            .collect::<Vec<&str>>()
            .iter()
            .find(|x| x.starts_with("version = "))
            .expect("Failed to get remote version")
            .trim_start_matches("version = \"")
            .trim_end_matches("\"");

        let local_version_str = env!("CARGO_PKG_VERSION");

        let remote_version: Vec<u8> = remote_version_str
            .split(".")
            .map(|x| x.parse::<u8>().expect("Failed to parse remote version"))
            .collect();
        let local_version: Vec<u8> = local_version_str
            .split(".")
            .map(|x| x.parse::<u8>().expect("Failed to parse remote version"))
            .collect();

        let mut out_of_date = false;

        for i in 0..local_version.len() {
            if remote_version[i] > local_version[i] {
                out_of_date = true;
                break;
            } else if remote_version[i] < local_version[i] {
                break;
            }
        }

        if out_of_date {
            if let Some(tx) = UPDATE_SIGNAL.lock().await.take() {
                printlg!("Starting update...");
                let _ = tx.send(());
                return Response::builder()
                    .status(StatusCode::OK)
                    .body(String::from("Starting update..."))
                    .unwrap();
            } else {
                printlg!("Update signal already sent");
                return Response::builder()
                    .status(StatusCode::CONFLICT)
                    .body(String::from("Update already sent!"))
                    .unwrap();
            }
        } else {
            printlg!("Already up to date!");
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(String::from("Already up to date!"))
                .unwrap();
        }
    } else {
        printlg!("Failed to make request to {}", REMOTE_CARGO_TOML_URL);
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(format!(
                "Failed to make request to {}",
                REMOTE_CARGO_TOML_URL
            ))
            .unwrap();
    }
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

fn key_create(l: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(l)
        .map(char::from)
        .collect()
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
