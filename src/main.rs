#[forbid(unsafe_code)]
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{
        header::{CONTENT_TYPE, LOCATION, SET_COOKIE},
        HeaderMap, HeaderName, HeaderValue, Response, StatusCode,
    },
    response::{Html, IntoResponse},
    routing::{get, head, post, put},
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::prelude::*;
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Instant, UNIX_EPOCH},
};
use tokio::{
    fs::{create_dir_all, read_dir, remove_dir_all, remove_file, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    signal,
    sync::Mutex,
};
use uuid::Uuid;

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
    static ref POPUPS: Arc<Mutex<Vec<(String, u64)>>> = Arc::new(Mutex::new(Vec::new()));
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
            downs_togo: Arc::new(Mutex::new(0)),
            countdown_text: Arc::new(Mutex::new(String::from("Countdown"))),
            show_countdown: Arc::new(Mutex::new(false)),
            show_downs: Arc::new(Mutex::new(true)),
            show_scoreboard: Arc::new(Mutex::new(true)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the application state
    let state = AppState::default();

    // Validate required files and directories
    if let Err(_) = File::open("secret.key").await {
        println!("Initializing secret.key");
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
        println!("Initializing config.json");
        let mut f = File::create("config.json")
            .await
            .expect("Cannot create config.json");

        let default_config = Config {
            secure_auth_cookie: true,
            sponsor_wait_time: 5,
            sponsor_img_height: 50,
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

    // Load sponsor img tags
    load_sponsors().await;

    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any) // Allow requests from any origin
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::HEAD]) // Allow specific methods
        .allow_headers(tower_http::cors::Any)
        .allow_private_network(true);

    let app = Router::new()
        // Basic routes
        .route("/", get(index_handler))
        .route("/", head(ping_handler))
        .route("/overlay", get(overlay_handler))
        .route("/settings", get(settings_handler))
        .route("/styles.css", get(css_handler))
        .route("/htmx.js", get(htmx_js_handler))
        .route("/app.js", get(app_js_handler))
        .route("/favicon.png", get(favicon_handler))
        .route("/spinner.svg", get(spinner_handler))
        // Login routes
        .route("/login", get(login_page_handler))
        .route("/login/", get(login_page_handler))
        .route("/login", post(login_handler))
        .route("/login/create", get(create_login_page_handler))
        .route("/login/create", post(create_login_handler))
        // Point routes
        .route("/home-points/update/:a", post(home_points_update_handler))
        .route("/home-points/display", put(home_points_display_handler))
        .route("/away-points/update/:a", post(away_points_update_handler))
        .route("/away-points/display", put(away_points_display_handler))
        // Game clock routes
        .route("/game-clock/display/:o", put(game_clock_display_handler))
        .route("/game-clock/ctl/:o", post(game_clock_ctl_handler))
        .route("/game-clock/set/:mins", post(game_clock_set_handler))
        .route(
            "/game-clock/update/:mins/:secs",
            post(game_clock_update_handler),
        )
        // Countdown clock routes
        .route(
            "/countdown-clock/display/:o",
            put(countdown_clock_display_handler),
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
        // Quarter routes
        .route("/quarter/display", put(quarter_display_handler))
        .route("/quarter/set/:q", post(quarter_set_handler))
        .route("/quarter/update/:a", post(quarter_update_handler))
        // Teaminfo routes
        .route("/teaminfo", get(teaminfo_handler))
        .route("/teaminfo/create", post(teaminfo_preset_create_handler))
        .route("/teaminfo/selector", put(teaminfo_preset_selector_handler))
        .route("/teaminfo/select/:id", post(teaminfo_preset_select_handler))
        .route("/teaminfo/remove/:id", post(teaminfo_preset_remove_handler))
        .route("/teaminfo/name/:t", put(team_name_display_handler))
        // Sponsor routes
        .route("/sponsors/upload", post(upload_sponsors_handler))
        .route("/sponsors/manage", put(sponsors_management_handler))
        .route("/sponsors/remove/:id", post(sponsor_remove_handler))
        .route("/sponsors/display", put(sponsor_display_handler))
        // Overlay display routes
        .route("/points/overview", put(points_overview_handler))
        .route("/icon/:t", put(icon_handler))
        .route("/overlay/clock", put(overlay_clock_handler))
        .route("/countdown/display", put(countdown_display_handler))
        // Downs routes
        .route("/downs/set/:d", post(downs_set_handler))
        .route("/downs/update/:d", post(downs_update_handler))
        .route("/downs/togo/set/:y", post(downs_togo_set_handler))
        .route("/downs/togo/update/:y", post(downs_togo_update_handler))
        .route("/downs/display/:t", put(downs_display_handler))
        // Visibility routes
        .route("/visibility/buttons", put(visibility_buttons_handler))
        .route("/visibility/toggle/:v", post(visibility_toggle_handler))
        .route("/visibility/css", put(visibility_css_handler))
        // OCR API
        .route("/ocr", post(ocr_handler))
        .route("/ocr/api/toggle", post(ocr_api_toggle_handler))
        .route("/ocr/api/button", put(ocr_api_button_handler))
        // API
        .route("/api/key/check/:k", post(api_key_check_handler))
        .route("/api/key/show", put(api_key_show_handler))
        .route("/api/key/reveal", post(api_key_reveal_handler))
        // Popups
        .route("/popup", post(popup_handler))
        .route("/popup/show", put(popup_show_handler))
        // Misc Handlers
        .route("/reset", post(reset_handler))
        // Information routes, state, and fallback
        .route(
            "/version",
            put(|| async { Html::from(env!("CARGO_PKG_VERSION")) }),
        )
        .route("/uptime-display", put(uptime_display_handler))
        .with_state(state)
        .fallback(get(not_found_handler))
        .layer(cors);

    if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        tokio::spawn(uptime_ticker());
        tokio::spawn(game_clock_ticker());
        tokio::spawn(countdown_clock_ticker());
        tokio::spawn(sponsor_ticker());
        tokio::spawn(popup_ticker());
        println!(" -> LISTENING ON: 0.0.0.0:3000");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("Could not serve app")?;
    } else {
        panic!("Could not bind tcp listener!");
    }

    println!("Shut down gracefully");

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct Config {
    secure_auth_cookie: bool,
    sponsor_wait_time: u64,
    sponsor_img_height: u16,
}

// region: basic pages

async fn index_handler(jar: CookieJar) -> impl IntoResponse {
    if verify_auth(jar).await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("./html/index.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login"))
            .body(String::new())
            .unwrap();
    }
}

async fn teaminfo_handler(jar: CookieJar) -> impl IntoResponse {
    if verify_auth(jar).await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("./html/teaminfo.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login"))
            .body(String::new())
            .unwrap();
    }
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

async fn settings_handler(jar: CookieJar) -> impl IntoResponse {
    if verify_auth(jar).await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("./html/settings.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login"))
            .body(String::new())
            .unwrap();
    }
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
// region: js routing

async fn htmx_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("./html/js/htmx.js")))
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
                .body(String::from("<p>Passwords do not match</p>"))
                .unwrap();
        }
    } else {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(
                "<p>Username and password cannot be empty or contain spaces</p>",
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
                        .body(String::from("Invalid login"))
                        .unwrap();
                }
            } else {
                return Response::builder()
                    .status(StatusCode::OK)
                    .body(String::from("Invalid login"))
                    .unwrap();
            }
        } else {
            return Response::builder()
                .status(StatusCode::OK)
                .body(String::from(
                    "<p>Username and password cannot be empty or contain spaces</p>",
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
    let mut secret = String::new();

    let secret_f = File::open("secret.key")
        .await
        .expect("Could not open secret.key");
    let mut buf_reader = BufReader::new(secret_f);

    if let Err(_) = buf_reader.read_to_string(&mut secret).await {
        panic!("Cannot read secret.key! Generating a auth token with an empty private key is unsecure!");
    };

    let claims = Claims {
        sub: Uuid::new_v4().to_string(),
        un: username,
        exp: (std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + std::time::Duration::from_secs(60 * 60 * 24 * 7).as_secs()) as usize,
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

async fn ping_handler() -> impl IntoResponse {
    return StatusCode::OK;
}

// endregion: login
// region: team routing

async fn home_points_update_handler(
    Path(a): Path<i32>,
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut home_points = state.home_points.lock().await;

        if *home_points as i32 + a >= 0 {
            *home_points = (*home_points as i32 + a) as u32;
        }

        // println!("home: {}", *home_points);

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn away_points_update_handler(
    Path(a): Path<i32>,
    jar: CookieJar,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut away_points = state.away_points.lock().await;

        if *away_points as i32 + a >= 0 {
            *away_points = (*away_points as i32 + a) as u32;
        }

        // println!("away: {}", *away_points);

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn home_points_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    Html::from(state.home_points.lock().await.to_string())
}

async fn away_points_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    Html::from(state.away_points.lock().await.to_string())
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

async fn uptime_display_handler() -> impl IntoResponse {
    let uptime = UPTIME_SECS.lock().await;

    Html::from(format!(
        "{}:{}:{}",
        *uptime / 3600,
        (*uptime % 3600) / 60,
        *uptime % 60
    ))
}

async fn game_clock_ticker() {
    loop {
        let call_time = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut game_clock = GAME_CLOCK.lock().await;
        let mut game_clock_start = GAME_CLOCK_START.lock().await;

        if *game_clock_start && !*OCR_API.lock().await {
            let time_diff = -1 * (Instant::now() - call_time).as_secs() as isize;
            if *game_clock as isize + time_diff >= 0 {
                *game_clock = (*game_clock as isize + time_diff) as usize;
            } else {
                *game_clock_start = false;
            }
        }
    }
}

async fn game_clock_ctl_handler(jar: CookieJar, Path(a): Path<String>) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut game_clock_start = GAME_CLOCK_START.lock().await;

        if a == "start" {
            *game_clock_start = true;
        } else if a == "stop" {
            *game_clock_start = false;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn game_clock_set_handler(jar: CookieJar, Path(mins): Path<usize>) -> impl IntoResponse {
    if verify_auth(jar).await {
        *GAME_CLOCK.lock().await = mins * 60;

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn game_clock_update_handler(
    jar: CookieJar,
    Path((mins, secs)): Path<(isize, isize)>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut game_clock = GAME_CLOCK.lock().await;
        let time_diff = mins * 60 + secs;

        if *game_clock as isize + time_diff >= 0 {
            *game_clock = (*game_clock as isize + time_diff) as usize;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn game_clock_display_handler(Path(o): Path<String>) -> impl IntoResponse {
    let game_clock = GAME_CLOCK.lock().await;
    let mut time_display = String::new();

    if o == "minutes" {
        time_display = (*game_clock / 60).to_string();
    } else if o == "seconds" {
        time_display = (*game_clock % 60).to_string();
    } else if o == "both" {
        time_display = format!("{}:{:02}", *game_clock / 60, *game_clock % 60);
    }

    Html::from(time_display)
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

async fn countdown_clock_ctl_handler(jar: CookieJar, Path(a): Path<String>) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut countdown_clock_start = COUNTDOWN_CLOCK_START.lock().await;

        if a == "start" {
            *countdown_clock_start = true;
        } else if a == "stop" {
            *countdown_clock_start = false;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn countdown_clock_set_handler(jar: CookieJar, Path(mins): Path<usize>) -> impl IntoResponse {
    if verify_auth(jar).await {
        *COUNTDOWN_CLOCK.lock().await = mins * 60;

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn countdown_clock_update_handler(
    jar: CookieJar,
    Path((mins, secs)): Path<(isize, isize)>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut coundown_clock = COUNTDOWN_CLOCK.lock().await;
        let time_diff = mins * 60 + secs;

        if *coundown_clock as isize + time_diff >= 0 {
            *coundown_clock = (*coundown_clock as isize + time_diff) as usize;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
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

    return Html::from(time_display);
}

#[derive(Deserialize)]
struct TextPayload {
    text: String,
}

async fn countdown_text_set_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Form(payload): Form<TextPayload>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        *state.countdown_text.lock().await = payload.text;

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

// endregion: time
// region: quarters

async fn quarter_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    let quarter = state.quarter.lock().await;

    let event_body = match *quarter {
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        _ => "OT",
    };

    Html::from(event_body)
}

async fn quarter_set_handler(
    jar: CookieJar,
    Path(q): Path<u8>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        *state.quarter.lock().await = q;

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn quarter_update_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(a): Path<i8>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut quarter = state.quarter.lock().await;

        if *quarter as i8 + a >= 1 && *quarter as i8 + a <= 5 {
            *quarter = (*quarter as i8 + a) as u8;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
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

async fn teaminfo_preset_create_handler(jar: CookieJar, mut form: Multipart) -> impl IntoResponse {
    if verify_auth(jar).await {
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

        let write_json =
            serde_json::to_string_pretty(&teaminfo).expect("Could not serialize teaminfo");

        let mut f = File::create(format!("team-presets/{}/teams.json", id))
            .await
            .expect("Could not create teams.json");
        f.write_all(write_json.as_bytes())
            .await
            .expect("Could not write to teams.json");

        return Response::builder()
            .status(StatusCode::OK)
            .header(
                HeaderName::from_static("hx-trigger"),
                HeaderValue::from_static("reload-selector"),
            )
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
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
                <img class=\"home-logo\" src=\"data:image/{};base64,{}\" alt=\"home-img\" height=\"30px\" width=\"auto\">
                <p class=\"teampreset-title\">{} vs {}</p>
                <img class=\"away-logo\" src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"30px\" width=\"auto\">
                <button class=\"select-button\" hx-post=\"/teaminfo/select/{}\" hx-swap=\"none\">Select</button>
                <button class=\"remove-button\" hx-post=\"/teaminfo/remove/{}\" hx-swap=\"none\">Remove</button>
            </div>",
                home_tag_type,
                BASE64_STANDARD.encode(home_img_bytes),
                teaminfo.home_name,
                teaminfo.away_name,
                away_tag_type,
                BASE64_STANDARD.encode(away_img_bytes),
                id,
                id
            );
        }
    }

    return Html::from(html);
}

async fn teaminfo_preset_select_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
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

                    *state.home_name.lock().await = team_info.home_name;
                    *state.away_name.lock().await = team_info.away_name;

                    break;
                }
            }
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn teaminfo_preset_remove_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        if let Ok(_) = remove_dir_all(format!("./team-presets/{}", id)).await {
            *state.preset_id.lock().await = String::new();
        }

        return Response::builder()
            .status(StatusCode::OK)
            .header(
                HeaderName::from_static("hx-trigger"),
                HeaderValue::from_static("reload-selector"),
            )
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
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

// endregion: teaminfo
// region: sponsors

async fn upload_sponsors_handler(jar: CookieJar, mut form: Multipart) -> impl IntoResponse {
    if verify_auth(jar).await {
        create_dir_all(format!("./sponsors"))
            .await
            .expect("Could not create sponsors directory");

        while let Some(field) = form
            .next_field()
            .await
            .expect("Could not get next field of sponsor multipart")
        {
            let mut f = File::create(format!(
                "./sponsors/{}.{}",
                id_create(12),
                field.file_name().unwrap().split(".").collect::<Vec<&str>>()[1]
            ))
            .await
            .expect("Could not create sponsor file");

            f.write_all(field.bytes().await.unwrap().as_ref())
                .await
                .expect("Could not write to sponsor file");
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
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
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

async fn sponsor_remove_handler(jar: CookieJar, Path(id): Path<String>) -> impl IntoResponse {
    if verify_auth(jar).await {
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

        return Response::builder()
            .status(StatusCode::OK)
            .header(
                HeaderName::from_static("hx-trigger"),
                HeaderValue::from_static("reload-sponsor"),
            )
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn load_sponsors() {
    create_dir_all(format!("./sponsors"))
        .await
        .expect("Could not create sponsors directory");

    let cfg = tokio::fs::read_to_string("./config.json")
        .await
        .expect("Could not read config json");
    let cfg_json: Config = serde_json::from_str(&cfg).expect("Could not deserialize config json");

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
            "<img src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"{}vh\" width=\"auto\">",
            mime_type,
            BASE64_STANDARD.encode(f_bytes),
            cfg_json.sponsor_img_height
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

async fn sponsor_display_handler() -> impl IntoResponse {
    if *SHOW_SPONSORS.lock().await {
        let sponsor_img = SPONSOR_TAGS.lock().await[*SPONSOR_IDX.lock().await].clone();
        return Html::from(format!(
            "<div class=\"ol-sponsor-parent\">{}</div>",
            sponsor_img
        ));
    } else {
        return Html::from(String::new());
    }
}

// endregion: sponsors
// region: overlay

async fn points_overview_handler(State(state): State<AppState>) -> impl IntoResponse {
    Html::from(format!(
        "<h3>{}-{}</h3>",
        state.home_points.lock().await,
        state.away_points.lock().await
    ))
}

// "<img class=\"ol-team-logo\" src=\"data:image/{};base64,{}\" height=\"30px\" width=\"auto\" alt=\"team-icon\">"

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

async fn overlay_clock_handler(State(state): State<AppState>) -> impl IntoResponse {
    let game_clock = GAME_CLOCK.lock().await;

    let quarter = state.quarter.lock().await;

    let quarter_display = match *quarter {
        1 => "1st",
        2 => "2nd",
        3 => "3rd",
        4 => "4th",
        _ => "OT",
    };

    Html::from(format!(
        "{}:{:02} - {}",
        *game_clock / 60,
        *game_clock % 60,
        quarter_display
    ))
}

async fn countdown_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    if *state.show_countdown.lock().await {
        let countdown_clock = COUNTDOWN_CLOCK.lock().await;
        return Html::from(format!(
            "<div id=\"ol-countdown\" class=\"countdown-container\"><h2 class=\"countdown-title\">{}:</h2>{}:{:02}</div>",
            state.countdown_text.lock().await,
            *countdown_clock / 60,
            *countdown_clock % 60
        ));
    } else {
        return Html::from(String::new());
    }
}

// endregion: overlay
// region: downs

async fn downs_set_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(d): Path<u8>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        if (1..=4).contains(&d) {
            *state.down.lock().await = d;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn downs_update_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(y): Path<i8>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut down = state.down.lock().await;
        if (1..=4).contains(&(*down as i8 + y)) {
            *down = (*down as i8 + y) as u8;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn downs_togo_set_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(y): Path<u8>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        *state.downs_togo.lock().await = y;

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn downs_togo_update_handler(
    jar: CookieJar,
    State(state): State<AppState>,
    Path(y): Path<i8>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut downs_togo = state.downs_togo.lock().await;
        if (0..=99).contains(&(*downs_togo as i8 + y)) {
            *downs_togo = (*downs_togo as i8 + y) as u8;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn downs_display_handler(
    State(state): State<AppState>,
    Path(t): Path<String>,
) -> impl IntoResponse {
    if t == "down" {
        let down = state.down.lock().await;

        return match *down {
            1 => Html::from(String::from("1st")),
            2 => Html::from(String::from("2nd")),
            3 => Html::from(String::from("3rd")),
            4 => Html::from(String::from("4th")),
            _ => Html::from(String::new()),
        };
    } else if t == "togo" {
        return Html::from(state.downs_togo.lock().await.to_string());
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

        return Html::from(format!(
            "{} & {}",
            down_display,
            *state.downs_togo.lock().await
        ));
    } else {
        return Html::from(String::new());
    }
}

// endregion: downs
// region: visibility

async fn visibility_buttons_handler(State(state): State<AppState>) -> impl IntoResponse {
    return Html::from(format!(
        "
    <div class=\"visibility-buttons\">
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
    jar: CookieJar,
    State(state): State<AppState>,
    Path(v): Path<String>,
) -> impl IntoResponse {
    if verify_auth(jar).await {
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
            }
            "downs" => {
                let mut downs = state.show_downs.lock().await;

                if *downs {
                    *downs = false;
                } else {
                    *downs = true;
                }
                modified = ("Downs/To Go", *downs);
            }
            "scoreboard" => {
                let mut scoreboard = state.show_scoreboard.lock().await;

                if *scoreboard {
                    *scoreboard = false;
                } else {
                    *scoreboard = true;
                }
                modified = ("Scoreboard", *scoreboard);
            }
            "sponsors" => {
                let mut sponsors = SHOW_SPONSORS.lock().await;

                if *sponsors {
                    *sponsors = false;
                } else {
                    *sponsors = true;
                }
                modified = ("Sponsors", *sponsors);
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
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn visibility_css_handler(State(state): State<AppState>) -> impl IntoResponse {
    return Html::from(format!(
        "
    <style>
        {}
        {}
    </style>",
        if !*state.show_downs.lock().await {
            ".downs { 
            display: none; 
        } 
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
    ));
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

            if api_key.to_str().expect("Could not cast HeaderVale to str") == login.api_key {
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

async fn ocr_api_toggle_handler(jar: CookieJar) -> impl IntoResponse {
    if verify_auth(jar).await {
        let mut ocr_api = OCR_API.lock().await;

        if !*ocr_api {
            *ocr_api = true;
        } else {
            *ocr_api = false;
        }

        return Response::builder()
            .status(StatusCode::OK)
            .body(format!(
                "{} OCR API",
                if !*ocr_api { "Enable" } else { "Disable" }
            ))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn ocr_api_button_handler() -> impl IntoResponse {
    return Html::from(format!(
        "<button hx-post=\"/ocr/api/toggle\">{} OCR API</button>",
        if !*OCR_API.lock().await {
            "Enable"
        } else {
            "Disable"
        }
    ));
}

async fn api_key_show_handler(jar: CookieJar) -> impl IntoResponse {
    if verify_auth(jar).await {
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
            .body(format!("<h6 id=\"api-key\">{}</h6><button onclick=\"apiCopy('{}')\">Copy</button>", hidden_key, login.api_key))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

async fn api_key_reveal_handler(jar: CookieJar) -> impl IntoResponse {
    if verify_auth(jar).await {
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
    } else {
        return Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(String::new())
            .unwrap();
    }
}

// endregion: api
// region: popups

async fn popup_handler(jar: CookieJar, Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    if verify_auth(jar).await {
        if let Some(p) = params.get("text") {
            POPUPS.lock().await.push((p.clone(), 7));
        }

        return StatusCode::OK;
    } else {
        return StatusCode::UNAUTHORIZED;
    }
}

async fn popup_ticker() {
    //let mut last_len = 0;
    loop {
        let start_time = Instant::now();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut popups = POPUPS.lock().await;

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
        //last_len = popups.len();
    }
}

async fn popup_show_handler() -> impl IntoResponse {
    let mut str_vec = Vec::new();

    for i in POPUPS.lock().await.clone() {
        str_vec.push(i.0);
    }

    let display = str_vec.join("<br>");

    return Html::from(display);
}

// endregion: popups
// region: misc

async fn reset_handler(jar: CookieJar, State(ref mut state): State<AppState>) -> impl IntoResponse {
    if verify_auth(jar).await {
        *state.home_points.lock().await = 0;
        *state.home_name.lock().await = String::from("Home");
        *state.away_points.lock().await = 0;
        *state.away_name.lock().await = String::from("Away");
        *state.quarter.lock().await = 1;
        *state.preset_id.lock().await = String::new();
        *state.down.lock().await = 1;
        *state.downs_togo.lock().await = 0;
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

        return StatusCode::OK;
    } else {
        return StatusCode::UNAUTHORIZED;
    }
}

// endregion: misc

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

// Code borrowed from https://github.com/tokio-rs/axum/blob/806bc26e62afc2e0c83240a9e85c14c96bc2ceb3/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
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
    }
}