#[forbid(unsafe_code)]
use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{
        header::{CONTENT_TYPE, LOCATION, SET_COOKIE},
        HeaderName, HeaderValue, Response, StatusCode,
    },
    response::{
        sse::{Event, KeepAlive},
        Html, IntoResponse, Sse,
    },
    routing::{get, post, put},
    Form, Router,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::prelude::*;
use futures_util::{stream, Stream};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use lazy_static::lazy_static;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{
    convert::Infallible,
    sync::Arc,
    time::{Instant, UNIX_EPOCH},
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    signal,
    sync::{Mutex, RwLock},
};
use tokio_stream::StreamExt as _;
use uuid::Uuid;

lazy_static! {
    static ref SHUTDOWN: Arc<RwLock<bool>> = Arc::new(RwLock::new(false));
    static ref UPTIME_SECS: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
}

#[derive(Clone)]
struct AppState {
    home_points: Arc<Mutex<u32>>,
    away_points: Arc<Mutex<u32>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the application state
    let state = AppState {
        home_points: Arc::new(Mutex::new(0)),
        away_points: Arc::new(Mutex::new(0)),
    };

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
        };

        f.write_all(
            serde_json::to_string_pretty(&default_config)
                .expect("Could not serialize default config")
                .as_bytes(),
        )
        .await
        .expect("Could not initialize config.json")
    }

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/overlay", get(overlay_handler))
        .route("/styles.css", get(css_handler))
        .route("/htmx.js", get(htmx_js_handler))
        .route("/sse.js", get(sse_js_handler))
        .route("/app.js", get(app_js_handler))
        .route("/favicon.png", get(favicon_handler))
        .route("/spinner.svg", get(spinner_handler))
        .route("/login", get(login_page_handler))
        .route("/login/", get(login_page_handler))
        .route("/login", post(login_handler))
        .route("/login/create", get(create_login_page_handler))
        .route("/login/create", post(create_login_handler))
        .route("/home-points/update/:a", post(home_points_update_handler))
        .route("/home-points/sse", get(home_points_sse_handler))
        .route("/away-points/update/:a", post(away_points_update_handler))
        .route("/away-points/sse", get(away_points_sse_handler))
        .route(
            "/version",
            put(|| async { Html::from(env!("CARGO_PKG_VERSION")) }),
        )
        .route("/uptime-sse", get(uptime_sse_handler))
        .with_state(state)
        .fallback(get(not_found_handler));

    if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        tokio::spawn(uptime_ticker());
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

async fn sse_js_handler() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/javascript")
        .body(String::from(include_str!("./html/js/sse.js")))
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

async fn login_handler(Form(data): Form<Login>) -> impl IntoResponse {
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

async fn home_points_sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let state = Arc::clone(&state.home_points);
    let shutdown_state = Arc::clone(&SHUTDOWN);

    let stream = stream::unfold((state, shutdown_state), |(state, shutdown_state)| async {
        let team_points = state.lock().await.clone().to_string();
        let shutdown = *shutdown_state.read().await;

        let event_type = if shutdown { "shutdown" } else { "message" };

        Some((
            Ok(Event::default().data(team_points).event(event_type)),
            (state, shutdown_state),
        ))
    })
    .throttle(tokio::time::Duration::from_millis(5));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn away_points_sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let state = Arc::clone(&state.away_points);
    let shutdown_state = Arc::clone(&SHUTDOWN);

    let stream = stream::unfold((state, shutdown_state), |(state, shutdown_state)| async {
        let team_points = state.lock().await.clone().to_string();
        let shutdown = *shutdown_state.read().await;

        let event_type = if shutdown { "shutdown" } else { "message" };

        Some((
            Ok(Event::default().data(team_points).event(event_type)),
            (state, shutdown_state),
        ))
    })
    .throttle(tokio::time::Duration::from_millis(5));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// endregion: team routing
// region: time

async fn uptime_ticker() {
    let start_time = Instant::now();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let uptime = (Instant::now() - start_time).as_secs() as usize;

        let mut uptime_secs = UPTIME_SECS.lock().await;
        *uptime_secs = uptime;
    }
}

async fn uptime_sse_handler() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let uptime_mutex = Arc::clone(&UPTIME_SECS);
    let shutdown_state = Arc::clone(&SHUTDOWN);

    let stream = stream::unfold(
        (uptime_mutex, shutdown_state),
        |(uptime_mutex, shutdown_state)| async {
            let shutdown = *shutdown_state.read().await;
            let uptime = *uptime_mutex.lock().await;

            let time_string = format!("{}:{}:{}", uptime / 3600, (uptime % 3600) / 60, uptime % 60);

            let event_type = if shutdown { "shutdown" } else { "message" };

            Some((
                Ok(Event::default().data(time_string).event(event_type)),
                (uptime_mutex, shutdown_state),
            ))
        },
    )
    .throttle(tokio::time::Duration::from_millis(5));

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// endregion: time

// Code borrowed from https://github.com/tokio-rs/axum/blob/806bc26e62afc2e0c83240a9e85c14c96bc2ceb3/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");

        let mut shutdown = SHUTDOWN.write().await;
        *shutdown = true;
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;

        let mut shutdown = SHUTDOWN.write().await;
        *shutdown = true;
    };

    #[cfg(not(unix))]
    let terminate = async {
        let _ = std::future::pending::<()>().await;
        let mut shutdown = SHUTDOWN.write().await;
        *shutdown = true;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}