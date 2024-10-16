use std::time::UNIX_EPOCH;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    http::{header::LOCATION, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Form,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use jsonwebtoken::{decode, DecodingKey, EncodingKey, Validation};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::header::SET_COOKIE;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
};
use uuid::Uuid;

use crate::Config;

pub const API_KEY_LEN: usize = 32;

pub async fn create_login_page_handler() -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login"))
            .body(String::new())
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("../html/create_login.html")))
            .unwrap();
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateLogin {
    username: String,
    password: String,
    confirm_password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Login {
    pub username: String,
    pub password: String,
    pub api_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginForm {
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

pub async fn create_login_handler(Form(data): Form<CreateLogin>) -> impl IntoResponse {
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
                api_key: key_create(API_KEY_LEN),
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

pub async fn login_page_handler() -> impl IntoResponse {
    if let Ok(_) = File::open("login.json").await {
        return Response::builder()
            .status(StatusCode::OK)
            .body(String::from(include_str!("../html/login.html")))
            .unwrap();
    } else {
        return Response::builder()
            .status(StatusCode::SEE_OTHER)
            .header(LOCATION, HeaderValue::from_static("/login/create"))
            .body(String::new())
            .unwrap();
    }
}

pub async fn login_handler(Form(data): Form<LoginForm>) -> impl IntoResponse {
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

pub async fn auth_cookie_builder(username: String) -> String {
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

pub async fn session_cookie_builder() -> String {
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

pub async fn verify_auth(jar: CookieJar) -> bool {
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

pub async fn verify_session(jar: CookieJar) -> bool {
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

pub async fn logout_handler() -> impl IntoResponse {
    return Response::builder()
        .status(StatusCode::OK)
        .header(SET_COOKIE, Cookie::build(("AuthToken", "0")).to_string())
        .header(SET_COOKIE, Cookie::build(("SessionToken", "0")).to_string())
        .header(
            HeaderName::from_static("hx-redirect"),
            HeaderValue::from_static("/"),
        )
        .body(String::new())
        .unwrap();
}

pub fn key_create(l: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(l)
        .map(char::from)
        .collect()
}
