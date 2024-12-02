// Froggi routing (misc)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::collections::HashMap;

use crate::{appstate::global::*, printlg, AppState};

pub async fn popup_handler(
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

pub async fn reset_handler(State(ref mut state): State<AppState>) -> impl IntoResponse {
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

pub async fn logs_handler() -> impl IntoResponse {
    let logs = LOGS.lock().await;
    let mut logs_display = Vec::new();

    for i in (0..logs.len()).rev() {
        logs_display.push(format!("<span>({}) {}</span>", i + 1, logs[i]))
    }

    Html::from(logs_display.join("<br>"))
}

pub async fn restart_handler() -> impl IntoResponse {
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

pub async fn shutdown_handler() -> impl IntoResponse {
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

pub async fn ping_handler() -> impl IntoResponse {
    return StatusCode::OK;
}
