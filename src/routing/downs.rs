use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{appstate::global::*, printlg, AppState};

pub async fn downs_set_handler(
    State(state): State<AppState>,
    Path(d): Path<u8>,
) -> impl IntoResponse {
    if (1..=4).contains(&d) {
        let mut down = state.down.lock().await;
        *down = d;

        printlg!("SET down: {}", *down);
    }

    return StatusCode::OK;
}

pub async fn downs_update_handler(
    State(state): State<AppState>,
    Path(y): Path<i8>,
) -> impl IntoResponse {
    let mut down = state.down.lock().await;

    let new_val = *down as i8 + y;

    if (1..=4).contains(&new_val) {
        *down = (*down as i8 + y) as u8;
        printlg!("UPDATE down: {}", *down);
    } else if new_val < 1 {
        *down = 4;
        printlg!("UPDATE down: {}", *down);
    } else if new_val > 4 {
        *down = 1;
        printlg!("UPDATE down: {}", *down);
    }

    return StatusCode::OK;
}

pub async fn downs_togo_set_handler(
    State(state): State<AppState>,
    Path(y): Path<u8>,
) -> impl IntoResponse {
    if (0..=101).contains(&y) {
        let mut togo = state.downs_togo.lock().await;
        *togo = y;

        printlg!("SET togo: {}", *togo);
    }

    return StatusCode::OK;
}

pub async fn downs_togo_update_handler(
    State(state): State<AppState>,
    Path(y): Path<i8>,
) -> impl IntoResponse {
    let mut downs_togo = state.downs_togo.lock().await;

    let new_val = *downs_togo as i8 + y;

    if (0..=101).contains(&new_val) {
        *downs_togo = new_val as u8;

        printlg!("UPDATE togo: {}", *downs_togo);
    } else if new_val > 101 {
        *downs_togo = 0;

        printlg!("UPDATE togo: {}", *downs_togo);
    } else if new_val < 0 {
        *downs_togo = 101;

        printlg!("UPDATE togo: {}", *downs_togo);
    }

    return StatusCode::OK;
}

pub async fn downs_display_handler(
    State(state): State<AppState>,
    Path(t): Path<String>,
) -> impl IntoResponse {
    if t == "down" {
        let down = state.down.lock().await;

        return match *down {
            1 => String::from("1st"),
            2 => String::from("2nd"),
            3 => String::from("3rd"),
            4 => String::from("4th"),
            _ => String::new(),
        };
    } else if t == "togo" {
        let downs_togo = state.downs_togo.lock().await;

        if *downs_togo == 101 {
            return String::from("Goal");
        } else if *downs_togo == 0 {
            return String::from("Hidden");
        } else {
            return downs_togo.to_string();
        }
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
        let downs_togo = state.downs_togo.lock().await;

        if *downs_togo != 0 {
            return format!(
                "{} & {}",
                down_display,
                if *downs_togo == 101 {
                    String::from("Goal")
                } else {
                    downs_togo.to_string()
                }
            );
        } else {
            return format!("{}", down_display);
        }
    } else {
        return String::new();
    }
}
