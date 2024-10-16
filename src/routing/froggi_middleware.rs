// Froggi middleware

use axum::{
    body::Body,
    extract::Request,
    http::{
        header::{LOCATION, SET_COOKIE},
        HeaderMap, HeaderValue, StatusCode,
    },
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;

use crate::utility::login::*;

pub async fn auth_session_layer(
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

pub async fn auth_give_session_layer(
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
