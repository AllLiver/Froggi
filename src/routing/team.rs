// Froggi routing (teams)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::appstate::global::*;
use crate::{printlg, AppState};

pub async fn home_points_update_handler(
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

pub async fn home_points_set_handler(
    Path(a): Path<u32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut home_points = state.home_points.lock().await;
    *home_points = a;

    printlg!("SET home_points: {}", *home_points);

    return StatusCode::OK;
}

pub async fn away_points_update_handler(
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

pub async fn away_points_set_handler(
    Path(a): Path<u32>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut away_points = state.away_points.lock().await;
    *away_points = a;

    printlg!("SET home_points: {}", *away_points);

    return StatusCode::OK;
}

pub async fn home_points_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.home_points.lock().await.to_string()
}

pub async fn away_points_display_handler(State(state): State<AppState>) -> impl IntoResponse {
    state.away_points.lock().await.to_string()
}
