// Froggi routing (time)

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Form,
};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{appstate::global::*, printlg, set_countdown_clock, set_game_clock, start_countdown_clock, start_game_clock, stop_countdown_clock, stop_game_clock, AppState};

pub async fn game_clock_ctl_handler(Path(a): Path<String>) -> impl IntoResponse {
    if a == "start" {
        start_game_clock().await;
    } else if a == "stop" {
        stop_game_clock().await;
    } else if a == "toggle" {
        if *GAME_CLOCK_START.lock().await {
            stop_game_clock().await;
        } else {
            start_game_clock().await;
        }
    }

    printlg!("UPDATE game_clock_start: {}", *GAME_CLOCK_START.lock().await);

    return StatusCode::OK;
}

pub async fn game_clock_set_handler(Path((mins, secs)): Path<(u64, u64)>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    set_game_clock(&mut game_clock, mins * 60 * 1000 + secs * 1000).await;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_set_mins_handler(Path(mins): Path<u64>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    let millis = (*game_clock / 1000 % 60 * 1000) + mins * 60 * 1000;
    set_game_clock(&mut game_clock, millis).await;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_set_secs_handler(Path(secs): Path<u64>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    let millis = (*game_clock / 1000 / 60 * 60 * 1000) + secs * 1000;
    set_game_clock(&mut game_clock, millis).await;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_update_handler(
    Path((mins, secs)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    let time_diff = mins * 60 * 1000 + secs * 1000;

    if *game_clock as i64 + time_diff >= 0 {
        let millis = (*game_clock as i64 + time_diff) as u64;
        set_game_clock(&mut game_clock, millis).await;
    }

    printlg!("UPDATE game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_display_handler(Path(o): Path<String>) -> impl IntoResponse {
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

pub async fn countdown_clock_ctl_handler(Path(a): Path<String>) -> impl IntoResponse {
    if a == "start" {
        start_countdown_clock().await;
    } else if a == "stop" {
        stop_countdown_clock().await;
    } else if a == "toggle" {
        if *COUNTDOWN_CLOCK_START.lock().await {
            stop_countdown_clock().await;
        } else {
            start_countdown_clock().await;
        }
    }

    printlg!("UPDATE countdown_clock_start: {}", *COUNTDOWN_CLOCK_START.lock().await);

    return StatusCode::OK;
}

pub async fn countdown_clock_set_handler(
    Path((mins, secs)): Path<(u64, u64)>,
) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    set_countdown_clock(&mut countdown_clock, mins * 60 * 1000 + secs * 1000).await;

    printlg!("SET countdown_clock: {}", *countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_set_mins_handler(Path(mins): Path<u64>) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    let millis = (*countdown_clock / 1000 % 60 * 1000) + mins * 60 * 1000;
    set_countdown_clock(&mut countdown_clock, millis).await;

    printlg!("SET game_clock: {}", *countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_set_secs_handler(Path(secs): Path<u64>) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    let millis = (*countdown_clock / 1000 / 60 * 60 * 1000) + secs * 1000;
    set_countdown_clock(&mut countdown_clock, millis).await;

    printlg!("SET countdown_clock: {}", *countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_update_handler(
    Path((mins, secs)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    let time_diff = mins * 60 * 1000 + secs * 1000;

    if *countdown_clock as i64 + time_diff >= 0 {
        let millis = (*countdown_clock as i64 + time_diff) as u64;
        set_countdown_clock(&mut countdown_clock, millis).await;
    }

    printlg!("UPDATE countdown_clock: {}", *countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_display_handler(Path(o): Path<String>) -> impl IntoResponse {
    let countdown_clock = COUNTDOWN_CLOCK.lock().await;
    let mut time_display = String::new();

    if o == "minutes" {
        time_display = (*countdown_clock / 1000 / 60).to_string();
    } else if o == "seconds" {
        time_display = (*countdown_clock / 1000 % 60).to_string();
    } else if o == "both" {
        time_display = format!(
            "{}:{:02}",
            *countdown_clock / 1000 / 60,
            *countdown_clock / 1000 % 60
        );
    }

    time_display
}

#[derive(Deserialize)]
pub struct TextPayload {
    text: String,
}

pub async fn countdown_text_set_handler(
    State(state): State<AppState>,
    Form(payload): Form<TextPayload>,
) -> impl IntoResponse {
    let mut countdown_text = state.countdown_text.lock().await;
    *countdown_text = payload.text;

    printlg!("SET countdown_text: {}", countdown_text);

    return StatusCode::OK;
}

pub async fn quarter_display_handler(State(state): State<AppState>) -> impl IntoResponse {
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

pub async fn quarter_set_handler(
    Path(q): Path<u8>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut quarter = state.quarter.lock().await;
    *quarter = q;

    printlg!("SET quarter: {}", *quarter);

    return StatusCode::OK;
}

pub async fn quarter_update_handler(
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