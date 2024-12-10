// Froggi routing (time)

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Form,
};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{appstate::global::*, printlg, AppState};

pub async fn game_clock_ctl_handler(Path(a): Path<String>) -> impl IntoResponse {
    let mut game_clock_start = GAME_CLOCK_START.lock().await;

    if a == "start" {
        *game_clock_start = true;
    } else if a == "stop" {
        *game_clock_start = false;
        let mut game_clock = GAME_CLOCK.lock().await;
        if *game_clock >= 1000 * 60 {
            *game_clock = *game_clock / 1000 * 1000;
        }
    } else if a == "toggle" {
        if *game_clock_start {
            *game_clock_start = false;
        } else {
            *game_clock_start = true;
        }
    }

    printlg!("UPDATE game_clock_start: {}", *game_clock_start);

    return StatusCode::OK;
}

pub async fn game_clock_set_handler(Path((mins, secs)): Path<(usize, usize)>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    *game_clock = mins * 60 * 1000 + secs * 1000;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_set_mins_handler(Path(mins): Path<usize>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    *game_clock = (*game_clock / 1000 % 60 * 1000) + mins * 60 * 1000;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_set_secs_handler(Path(secs): Path<usize>) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    *game_clock = (*game_clock / 1000 / 60 * 60 * 1000) + secs * 1000;

    printlg!("SET game_clock: {}", game_clock);

    return StatusCode::OK;
}

pub async fn game_clock_update_handler(
    Path((mins, secs)): Path<(isize, isize)>,
) -> impl IntoResponse {
    let mut game_clock = GAME_CLOCK.lock().await;
    let time_diff = mins * 60 * 1000 + secs * 1000;

    if *game_clock as isize + time_diff >= 0 {
        *game_clock = (*game_clock as isize + time_diff) as usize;
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
    let mut countdown_clock_start = COUNTDOWN_CLOCK_START.lock().await;

    if a == "start" {
        *countdown_clock_start = true;
    } else if a == "stop" {
        *countdown_clock_start = false;
    } else if a == "toggle" {
        if *countdown_clock_start {
            *countdown_clock_start = false;
        } else {
            *countdown_clock_start = true;
        }
    }

    printlg!("UPDATE countdown_clock_start: {}", *countdown_clock_start);

    return StatusCode::OK;
}

pub async fn countdown_clock_set_handler(
    Path((mins, secs)): Path<(usize, usize)>,
) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    *countdown_clock = mins * 60 * 1000 + secs * 1000;

    printlg!("SET countdown_clock: {}", *countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_set_mins_handler(Path(mins): Path<usize>) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    *countdown_clock = (*countdown_clock / 1000 % 60 * 1000) + mins * 60 * 1000;

    printlg!("SET game_clock: {}", countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_set_secs_handler(Path(secs): Path<usize>) -> impl IntoResponse {
    let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
    *countdown_clock = (*countdown_clock / 1000 / 60 * 60 * 1000) + secs * 1000;

    printlg!("SET countdown_clock: {}", countdown_clock);

    return StatusCode::OK;
}

pub async fn countdown_clock_update_handler(
    Path((mins, secs)): Path<(isize, isize)>,
) -> impl IntoResponse {
    let mut coundown_clock = COUNTDOWN_CLOCK.lock().await;
    let time_diff = mins * 60 * 1000 + secs * 1000;

    if *coundown_clock as isize + time_diff >= 0 {
        *coundown_clock = (*coundown_clock as isize + time_diff) as usize;
    }

    printlg!("UPDATE countdown_clock: {}", *coundown_clock);

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