// Froggi routing 

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    response::Html,
    routing::{get, head, post, put},
    Router,
};
use tower_http::cors::CorsLayer;

mod api;
mod basic;
mod downs;
mod froggi_middleware;
mod login;
mod misc;
mod overlay;
mod sponsors;
mod team;
mod teaminfo;
mod time;
mod updating;
mod visibility;
mod websockets;

use api::*;
use basic::*;
use downs::*;
use froggi_middleware::*;
use login::*;
use misc::*;
use overlay::*;
use sponsors::*;
use team::*;
use teaminfo::*;
use time::*;
use updating::*;
use visibility::*;
use websockets::*;

use crate::AppState;

pub fn froggi_router(state: &AppState) -> Router {
    // Set up CORS
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any) // Allow requests from any origin
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::HEAD,
        ]) // Allow specific methods
        .allow_headers(tower_http::cors::Any)
        .allow_private_network(true);

    let auth_give_session_routes = Router::new()
        .route("/", get(index_handler))
        .route("/teaminfo", get(teaminfo_handler))
        .route("/settings", get(settings_handler))
        .layer(middleware::from_fn(auth_give_session_layer));

    let auth_session_routes = Router::new()
        .route("/dashboard-websocket", get(dashboard_websocket_handler))
        .route("/home-points/update/:a", post(home_points_update_handler))
        .route("/home-points/set/:a", post(home_points_set_handler))
        .route("/away-points/update/:a", post(away_points_update_handler))
        .route("/away-points/set/:a", post(away_points_set_handler))
        .route("/game-clock/ctl/:o", post(game_clock_ctl_handler))
        .route("/game-clock/set/:mins/:secs", post(game_clock_set_handler))
        .route("/game-clock/set-mins/:mins", post(game_clock_set_mins_handler))
        .route("/game-clock/set-secs/:secs", post(game_clock_set_secs_handler))
        .route(
            "/game-clock/update/:mins/:secs",
            post(game_clock_update_handler),
        )
        .route("/countdown-clock/ctl/:o", post(countdown_clock_ctl_handler))
        .route("/countdown-clock/set-mins/:mins", post(countdown_clock_set_mins_handler))
        .route("/countdown-clock/set-secs/:secs", post(countdown_clock_set_secs_handler))
        .route(
            "/countdown-clock/set/:mins/:secs",
            post(countdown_clock_set_handler),
        )
        .route(
            "/countdown-clock/update/:mins/:secs",
            post(countdown_clock_update_handler),
        )
        .route("/countdown/text/set", post(countdown_text_set_handler))
        .route("/quarter/set/:q", post(quarter_set_handler))
        .route("/quarter/update/:a", post(quarter_update_handler))
        .route("/teaminfo/create", post(teaminfo_preset_create_handler))
        .route("/teaminfo/select/:id", post(teaminfo_preset_select_handler))
        .route("/teaminfo/remove/:id", post(teaminfo_preset_remove_handler))
        .route(
            "/sponsors/upload",
            post(upload_sponsors_handler).layer(DefaultBodyLimit::max(2000000000)),
        )
        .route("/sponsors/remove/:id", post(sponsor_remove_handler))
        .route("/downs/set/:d", post(downs_set_handler))
        .route("/downs/update/:d", post(downs_update_handler))
        .route("/downs/togo/set/:y", post(downs_togo_set_handler))
        .route("/downs/togo/update/:y", post(downs_togo_update_handler))
        .route("/visibility/toggle/:v", post(visibility_toggle_handler))
        .route("/ocr/api/toggle", post(ocr_api_toggle_handler))
        .route("/api/key/show", put(api_key_show_handler))
        .route("/api/key/reveal", post(api_key_reveal_handler))
        .route("/api/key/regen", post(api_key_regen_handler))
        .route("/popup/:t", post(popup_handler))
        .route("/reset", post(reset_handler))
        .route("/restart", post(restart_handler))
        .route("/shutdown", post(shutdown_handler))
        .route("/update", post(update_handler))
        .route("/logout", post(logout_handler))
        .layer(middleware::from_fn(auth_session_layer));

    let app = Router::new()
        .route("/", head(ping_handler))
        .route("/overlay", get(overlay_handler))
        .route("/styles.css", get(css_handler))
        .route("/htmx.js", get(htmx_js_handler))
        .route("/app.js", get(app_js_handler))
        .route("/ws.js", get(ws_js_handler))
        .route("/favicon.png", get(favicon_handler))
        .route("/overlay-websocket", get(overlay_websocket_handler))
        .route("/login", get(login_page_handler))
        .route("/login/", get(login_page_handler))
        .route("/login", post(login_handler))
        .route("/login/create", get(create_login_page_handler))
        .route("/login/create", post(create_login_handler))
        .route("/home-points/display", get(home_points_display_handler))
        .route("/away-points/display", get(away_points_display_handler))
        .route("/game-clock/display/:o", get(game_clock_display_handler))
        .route(
            "/countdown-clock/display/:o",
            get(countdown_clock_display_handler),
        )
        .route("/quarter/display", get(quarter_display_handler))
        .route("/teaminfo/selector", put(teaminfo_preset_selector_handler))
        .route("/teaminfo/name/:t", put(team_name_display_handler))
        .route("/teaminfo/button-css", put(teaminfo_button_css_handler))
        .route("/sponsors/manage", put(sponsors_management_handler))
        .route("/icon/:t", put(icon_handler))
        .route(
            "/overlay/team-border-css",
            put(overlay_team_border_css_handler),
        )
        .route("/downs/display/:t", get(downs_display_handler))
        .route("/visibility/buttons", put(visibility_buttons_handler))
        .route("/ocr", post(ocr_handler))
        .route("/ocr/api/button", put(ocr_api_button_handler))
        .route("/api/key/check/:k", post(api_key_check_handler))
        .route("/logs", put(logs_handler))
        .route(
            "/version",
            put(|| async { Html::from(env!("CARGO_PKG_VERSION")) }),
        )
        .route("/update/menu", put(update_menu_handler))
        .nest("/", auth_session_routes)
        .nest("/", auth_give_session_routes)
        .with_state(state.clone())
        .fallback(get(not_found_handler))
        .layer(cors);

    app
}
