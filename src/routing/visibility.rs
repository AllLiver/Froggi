use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};

use crate::{appstate::global::*, printlg, AppState};

pub async fn visibility_buttons_handler(State(state): State<AppState>) -> impl IntoResponse {
    return Html::from(format!(
        "
    <div class=\"display-buttons\">
        <button class=\"show-countdown\" hx-post=\"/visibility/toggle/countdown\">{}</button>
        <button class=\"show-downs\" hx-post=\"/visibility/toggle/downs\">{}</button>
        <button class=\"show-scoreboard\" hx-post=\"/visibility/toggle/scoreboard\">{}</button>
        <button class=\"show-sponsors\" hx-post=\"/visibility/toggle/sponsors\">{}</button>
    </div>
    ",
        if !*state.show_countdown.lock().await {
            "Show Countdown"
        } else {
            "Hide Countdown"
        },
        if !*state.show_downs.lock().await {
            "Show Downs/To Go"
        } else {
            "Hide Downs/To Go"
        },
        if !*state.show_scoreboard.lock().await {
            "Show Scoreboard"
        } else {
            "Hide Scoreboard"
        },
        if !*SHOW_SPONSORS.lock().await {
            "Show Sponsors"
        } else {
            "Hide Sponsors"
        }
    ));
}

pub async fn visibility_toggle_handler(
    State(state): State<AppState>,
    Path(v): Path<String>,
) -> impl IntoResponse {
    let mut modified = ("", false);

    match v.as_str() {
        "countdown" => {
            let mut countdown = state.show_countdown.lock().await;

            if *countdown {
                *countdown = false;
            } else {
                *countdown = true;
            }
            modified = ("Countdown", *countdown);

            printlg!("SET {} visibility: {}", modified.0, *countdown);
        }
        "downs" => {
            let mut downs = state.show_downs.lock().await;

            if *downs {
                *downs = false;
            } else {
                *downs = true;
            }
            modified = ("Downs/To Go", *downs);

            printlg!("SET {} visibility: {}", modified.0, *downs);
        }
        "scoreboard" => {
            let mut scoreboard = state.show_scoreboard.lock().await;

            if *scoreboard {
                *scoreboard = false;
            } else {
                *scoreboard = true;
            }
            modified = ("Scoreboard", *scoreboard);

            printlg!("SET {} visibility: {}", modified.0, *scoreboard);
        }
        "sponsors" => {
            let mut sponsors = SHOW_SPONSORS.lock().await;

            if *sponsors {
                *sponsors = false;
            } else {
                *sponsors = true;
            }
            modified = ("Sponsors", *sponsors);

            printlg!("SET {} visibility: {}", modified.0, *sponsors);
        }
        _ => {}
    }

    return Response::builder()
        .status(StatusCode::OK)
        .body(format!(
            "{} {}",
            if !modified.1 { "Show" } else { "Hide" },
            modified.0
        ))
        .unwrap();
}
