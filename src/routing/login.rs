// Froggi routing (login)

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    http::{header::LOCATION, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Form,
};
use axum_extra::extract::cookie::Cookie;
use reqwest::header::SET_COOKIE;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
};

use crate::{utility::login::*, CreateLogin, Login};

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
pub struct LoginForm {
    username: String,
    password: String,
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
