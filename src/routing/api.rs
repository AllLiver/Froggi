use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};

use crate::{appstate::global::*, key_create, printlg, AppState, Login, API_KEY_LEN};

pub async fn api_key_check_handler(Path(k): Path<String>) -> impl IntoResponse {
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

pub async fn ocr_handler(
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

pub async fn ocr_api_toggle_handler() -> impl IntoResponse {
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

pub async fn ocr_api_button_handler() -> impl IntoResponse {
    return Html::from(format!(
        "<button class=\"button-settings\" hx-post=\"/ocr/api/toggle\">{} OCR API</button>",
        if !*OCR_API.lock().await {
            "Enable"
        } else {
            "Disable"
        }
    ));
}

pub async fn api_key_show_handler() -> impl IntoResponse {
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
        <button hx-post=\"/api/key/regen\" hx-swap=\"none\" class=\"button-settings\" hx-confirm=\"This will reset the API key to a new, random API key, are you sure?\">Regenerate Api Key</button>
        </div>",
        hidden_key, login.api_key
    ))
    .unwrap();
}

pub async fn api_key_reveal_handler() -> impl IntoResponse {
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

pub async fn api_key_regen_handler() -> impl IntoResponse {
    let current_login: Login = serde_json::from_str(
        &tokio::fs::read_to_string("./login.json")
            .await
            .expect("Failed to read login.json"),
    )
    .expect("Failed to deserialize login.json");

    tokio::fs::write(
        "./login.json",
        serde_json::to_string(&Login {
            api_key: key_create(API_KEY_LEN),
            ..current_login
        })
        .expect("Failed to serialize login.json")
        .as_bytes(),
    )
    .await
    .expect("Failed to write to login.json");

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("hide-api-key"),
        )
        .body(String::new())
        .unwrap();
}
