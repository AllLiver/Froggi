// Froggi appstate

use std::sync::Arc;
use tokio::sync::Mutex;

pub mod global {
    use super::*;
    use lazy_static::lazy_static;
    use tokio::sync::oneshot;

    lazy_static! {
        pub static ref GAME_CLOCK: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        pub static ref GAME_CLOCK_START: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        pub static ref COUNTDOWN_CLOCK: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        pub static ref COUNTDOWN_CLOCK_START: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        pub static ref LOGS: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        pub static ref SPONSOR_TAGS: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        pub static ref SPONSOR_IDX: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        pub static ref SHOW_SPONSORS: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        pub static ref OCR_API: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        pub static ref POPUPS_HOME: Arc<Mutex<Vec<(String, u64)>>> =
            Arc::new(Mutex::new(Vec::new()));
        pub static ref POPUPS_AWAY: Arc<Mutex<Vec<(String, u64)>>> =
            Arc::new(Mutex::new(Vec::new()));
        pub static ref COUNTDOWN_OPACITY: Arc<Mutex<f32>> = Arc::new(Mutex::new(1.0));
        pub static ref RESTART_SIGNAL: Arc<Mutex<Option<oneshot::Sender<()>>>> =
            Arc::new(Mutex::new(None));
        pub static ref SHUTDOWN_SIGNAL: Arc<Mutex<Option<oneshot::Sender<()>>>> =
            Arc::new(Mutex::new(None));
        pub static ref UPDATE_SIGNAL: Arc<Mutex<Option<oneshot::Sender<()>>>> =
            Arc::new(Mutex::new(None));
        pub static ref EXIT_CODE: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));
        pub static ref OUT_OF_DATE: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
        pub static ref REMOTE_VERSION: Arc<Mutex<String>> =
            Arc::new(Mutex::new(String::from("0.0.0")));
    }
}

pub mod routing {
    use std::time::Instant;

    use super::global::*;
    use super::*;
    use anyhow::{anyhow, Error};
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug)]
    pub struct AppState {
        pub home_points: Arc<Mutex<u32>>,
        pub home_name: Arc<Mutex<String>>,
        pub away_points: Arc<Mutex<u32>>,
        pub away_name: Arc<Mutex<String>>,
        pub quarter: Arc<Mutex<u8>>,
        pub preset_id: Arc<Mutex<String>>,
        pub down: Arc<Mutex<u8>>,
        pub downs_togo: Arc<Mutex<u8>>,
        pub countdown_text: Arc<Mutex<String>>,
        pub show_countdown: Arc<Mutex<bool>>,
        pub show_downs: Arc<Mutex<bool>>,
        pub show_scoreboard: Arc<Mutex<bool>>,
        pub start_time: Arc<Mutex<Instant>>,
    }

    impl AppState {
        pub fn default() -> AppState {
            AppState {
                home_points: Arc::new(Mutex::new(0)),
                home_name: Arc::new(Mutex::new(String::from("Home"))),
                away_points: Arc::new(Mutex::new(0)),
                away_name: Arc::new(Mutex::new(String::from("Away"))),
                quarter: Arc::new(Mutex::new(1)),
                preset_id: Arc::new(Mutex::new(String::new())),
                down: Arc::new(Mutex::new(1)),
                downs_togo: Arc::new(Mutex::new(1)),
                countdown_text: Arc::new(Mutex::new(String::from("Countdown"))),
                show_countdown: Arc::new(Mutex::new(false)),
                show_downs: Arc::new(Mutex::new(true)),
                show_scoreboard: Arc::new(Mutex::new(true)),
                start_time: Arc::new(Mutex::new(Instant::now())),
            }
        }
        pub async fn load_saved_state() -> Result<AppState, Error> {
            if let Ok(contents) = tokio::fs::read_to_string("./appstate.json").await {
                if let Ok(saved_state) = serde_json::from_str::<AppStateSerde>(&contents) {
                    *SHOW_SPONSORS.lock().await = saved_state.show_sponsors;
                    *OCR_API.lock().await = saved_state.ocr_api;
                    *GAME_CLOCK.lock().await = saved_state.game_clock;
                    *COUNTDOWN_CLOCK.lock().await = saved_state.countdown_clock;

                    return Ok(AppState {
                        home_points: Arc::new(Mutex::new(saved_state.home_points)),
                        home_name: Arc::new(Mutex::new(saved_state.home_name)),
                        away_points: Arc::new(Mutex::new(saved_state.away_points)),
                        away_name: Arc::new(Mutex::new(saved_state.away_name)),
                        quarter: Arc::new(Mutex::new(saved_state.quarter)),
                        preset_id: Arc::new(Mutex::new(saved_state.preset_id)),
                        down: Arc::new(Mutex::new(saved_state.down)),
                        downs_togo: Arc::new(Mutex::new(saved_state.downs_togo)),
                        countdown_text: Arc::new(Mutex::new(saved_state.countdown_text)),
                        show_countdown: Arc::new(Mutex::new(saved_state.show_countdown)),
                        show_downs: Arc::new(Mutex::new(saved_state.show_downs)),
                        show_scoreboard: Arc::new(Mutex::new(saved_state.show_scoreboard)),
                        start_time: Arc::new(Mutex::new(Instant::now())),
                    });
                } else {
                    return Err(anyhow!("Failed to deserialize appstate.json"));
                }
            } else {
                return Err(anyhow!("Failed to open appstate.json"));
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct AppStateSerde {
        home_points: u32,
        home_name: String,
        away_points: u32,
        away_name: String,
        quarter: u8,
        preset_id: String,
        down: u8,
        downs_togo: u8,
        countdown_text: String,
        show_countdown: bool,
        show_downs: bool,
        show_scoreboard: bool,
        show_sponsors: bool,
        ocr_api: bool,
        game_clock: usize,
        countdown_clock: usize,
    }

    impl AppStateSerde {
        pub async fn consume_app_state(state: AppState) -> AppStateSerde {
            AppStateSerde {
                home_points: *state.home_points.lock().await,
                home_name: state.home_name.lock().await.clone(),
                away_points: *state.away_points.lock().await,
                away_name: state.away_name.lock().await.clone(),
                quarter: *state.quarter.lock().await,
                preset_id: state.preset_id.lock().await.clone(),
                down: *state.down.lock().await,
                downs_togo: *state.downs_togo.lock().await,
                countdown_text: state.countdown_text.lock().await.clone(),
                show_countdown: *state.show_countdown.lock().await,
                show_downs: *state.show_downs.lock().await,
                show_scoreboard: *state.show_scoreboard.lock().await,
                show_sponsors: *SHOW_SPONSORS.lock().await,
                ocr_api: *OCR_API.lock().await,
                game_clock: *GAME_CLOCK.lock().await,
                countdown_clock: *COUNTDOWN_CLOCK.lock().await,
            }
        }
    }
}
