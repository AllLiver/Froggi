// Froggi routing (websockets)

use axum::{
    extract::{ws::Message, State, WebSocketUpgrade},
    response::IntoResponse,
};
use rand::{thread_rng, Rng};

use crate::appstate::global::*;
use crate::appstate::routing::*;

const WEBSOCKET_UPDATE_MILLIS: u64 = 100;

pub async fn dashboard_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(
            WEBSOCKET_UPDATE_MILLIS + thread_rng().gen_range(5..=20),
        ));
        loop {
            interval.tick().await;

            let game_clock = GAME_CLOCK.lock().await;
            let countdown_clock = COUNTDOWN_CLOCK.lock().await;

            let message = format!(
                "
            <div id=\"home-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"away-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"clock-counter-minutes\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"clock-counter-seconds\" hx-swap-oob=\"innerHTML\">{:02}</div>
            <div id=\"countdown-counter-minutes\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"countdown-counter-seconds\" hx-swap-oob=\"innerHTML\">{:02}</div>
            <div id=\"downs-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"togo-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"quarter-counter\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"uptime-value\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"backend-css-index\" hx-swap-oob=\"innerHTML\">{}</div>
            ",
                *state.home_points.lock().await,
                *state.away_points.lock().await,
                *game_clock / 1000 / 60,
                *game_clock / 1000 % 60,
                *countdown_clock / 1000 / 60,
                *countdown_clock / 1000 % 60,
                match *state.down.lock().await {
                    1 => "1st",
                    2 => "2nd",
                    3 => "3rd",
                    4 => "4th",
                    _ => "",
                },
                {
                    let downs_togo = state.downs_togo.lock().await;

                    if *downs_togo == 0 {
                        String::from("Hidden")
                    } else if *downs_togo == 101 {
                        String::from("Goal")
                    } else {
                        downs_togo.to_string()
                    }
                },
                match *state.quarter.lock().await {
                    1 => "1st",
                    2 => "2nd",
                    3 => "3rd",
                    4 => "4th",
                    _ => "OT",
                },
                {
                    let uptime = UPTIME_SECS.lock().await;

                    format!(
                        "{:02}:{:02}:{:02}",
                        *uptime / 3600,
                        (*uptime % 3600) / 60,
                        *uptime % 60
                    )
                },
                format!(
                    "
            <style>
                {}
            </style>",
                    if !*state.show_downs.lock().await {
                        ".downs { 
                        display: none; 
                    }"
                    } else {
                        ""
                    },
                )
            );

            if socket.send(Message::Text(message)).await.is_err() {
                return;
            }
        }
    })
}

pub async fn overlay_websocket_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|mut socket| async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_millis(WEBSOCKET_UPDATE_MILLIS - thread_rng().gen_range(5..=20)));

        let countdown_opacity = COUNTDOWN_OPACITY.lock().await.clone();

        loop {
            interval.tick().await;

            let game_clock = GAME_CLOCK.lock().await;

            let message = format!(
            "
            <div id=\"ol-score\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"ol-time\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"ol-down\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"countdown-box\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"sponsor-roll\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"backend-css-overlay\" hx-swap-oob=\"innerHTML\">{}</div>
            <div id=\"ol-popupContainer\" hx-swap-oob=\"innerHTML\">{}</div>
            ",
            format!("{}-{}", *state.home_points.lock().await, *state.away_points.lock().await),
            {
                let quarter_display = match *state.quarter.lock().await {
                    1 => "1st",
                    2 => "2nd",
                    3 => "3rd",
                    4 => "4th",
                    _ => "OT",
                };
                if *game_clock >= 1000 * 60 {
                    format!(
                        "{}:{:02} - {}",
                        *game_clock / 1000 / 60,
                        *game_clock / 1000 % 60,
                        quarter_display
                    )
                } else {
                    format!(
                        "{}:{:02}:{:02} - {}",
                        *game_clock / 1000 / 60,
                        *game_clock / 1000 % 60,
                        *game_clock / 10 % 100,
                        quarter_display
                    )
                }
            },
            {
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
                    format!(
                        "{} & {}",
                        down_display,
                        if *downs_togo == 101 {
                            String::from("Goal")
                        } else {
                            downs_togo.to_string()
                        }
                    )
                } else {
                    format!("{}", down_display,)
                }
            },
            {
                if *state.show_countdown.lock().await {
                    let countdown_clock = COUNTDOWN_CLOCK.lock().await;
                    format!(
                        "<div id=\"ol-countdown\" class=\"countdown-container\" style=\"opacity: {};\"><h2 class=\"countdown-title\">{}:</h2>{}:{:02}</div>",
                        countdown_opacity,
                        state.countdown_text.lock().await,
                        *countdown_clock / 1000 / 60,
                        *countdown_clock / 1000 % 60
                    )
                } else {
                    String::new()
                }
            },
            {
                let sponsor_tags = SPONSOR_TAGS.lock().await;
                if *SHOW_SPONSORS.lock().await && sponsor_tags.len() > 0 {
                    let sponsor_img = sponsor_tags[*SPONSOR_IDX.lock().await].clone();
                    format!(
                        "<div class=\"ol-sponsor-parent\">{}</div>",
                        sponsor_img
                    )
                } else {
                    String::new()
                }
            },
            {
                format!(
                    "
                <style>
                    {}
                    {}
                </style>",
                    if !*state.show_downs.lock().await {
                    "
                    .ol-down-togo-box { 
                        display: none; 
                    }"
                    } else {
                        ""
                    },
                    if !*state.show_scoreboard.lock().await {
                        ".ol-parent-container { display: none; }"
                    } else {
                        ""
                    }
                )
            },
            {
                let mut html = String::new();

                let popups_home = POPUPS_HOME.lock().await;
                let mut h_vec = Vec::new();

                for i in 0..popups_home.len() {
                    h_vec.push(format!("<span>{}</span>", popups_home[i].0));
                }

                if h_vec.len() > 0 {
                    html += &format!("<div class=\"ol-home-popup\">{}</div>", h_vec.join("<br>"));
                }

                drop(h_vec);

                let popups_away = POPUPS_AWAY.lock().await;
                let mut a_vec = Vec::new();

                for i in 0..popups_away.len() {
                    a_vec.push(format!("<span>{}</span>", popups_away[i].0));
                }

                if a_vec.len() > 0 {
                    html += &format!("<div class=\"ol-away-popup\">{}</div>", a_vec.join("<br>"));
                }

                drop(a_vec);

                html
            }
            );

            if socket.send(Message::Text(message)).await.is_err() {
                return;
            }
        }
    })
}
