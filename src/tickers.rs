// Froggi tickers

use std::time::Instant;

use crate::{appstate::global::*, update_checker, utility::Config};

pub async fn popup_home_ticker() {
    loop {
        let start_time = Instant::now();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut popups = POPUPS_HOME.lock().await;

        let mut i = 0;
        loop {
            if i >= popups.len() {
                break;
            }

            let time_diff = (Instant::now() - start_time).as_secs();
            if popups[i].1 - time_diff > 0 {
                popups[i].1 -= time_diff;
                i += 1;
            } else {
                popups.remove(i);
            }
        }
    }
}

pub async fn popup_away_ticker() {
    loop {
        let start_time = Instant::now();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let mut popups = POPUPS_AWAY.lock().await;

        let mut i = 0;
        loop {
            if i >= popups.len() {
                break;
            }

            let time_diff = (Instant::now() - start_time).as_secs();
            if popups[i].1 - time_diff > 0 {
                popups[i].1 -= time_diff;
                i += 1;
            } else {
                popups.remove(i);
            }
        }
    }
}

pub async fn sponsor_ticker() {
    let cfg = tokio::fs::read_to_string("./config.json")
        .await
        .expect("Could not read config json");
    let cfg_json: Config = serde_json::from_str(&cfg).expect("Could not deserialize config json");

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(cfg_json.sponsor_wait_time)).await;
        let mut sponsor_idx = SPONSOR_IDX.lock().await;
        let show_sponsors = SHOW_SPONSORS.lock().await;

        let sponsor_tags_len = SPONSOR_TAGS.lock().await.len();

        if *show_sponsors && sponsor_tags_len > 0 {
            if *sponsor_idx < sponsor_tags_len - 1 {
                *sponsor_idx += 1;
            } else {
                *sponsor_idx = 0;
            }
        }
    }
}

pub async fn countdown_clock_ticker() {
    loop {
        let call_time = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut countdown_clock = COUNTDOWN_CLOCK.lock().await;
        let mut countdown_clock_start = COUNTDOWN_CLOCK_START.lock().await;

        if *countdown_clock_start {
            let time_diff = -1 * (Instant::now() - call_time).as_secs() as isize;
            if *countdown_clock as isize + time_diff >= 0 {
                *countdown_clock = (*countdown_clock as isize + time_diff) as usize;
            } else {
                *countdown_clock_start = false;
            }
        }
    }
}

pub async fn uptime_ticker() {
    let start_time = Instant::now();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        let mut uptime_secs = UPTIME_SECS.lock().await;

        *uptime_secs = (Instant::now() - start_time).as_secs() as usize;
    }
}

pub async fn game_clock_ticker() {
    loop {
        let call_time = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        let mut game_clock = GAME_CLOCK.lock().await;
        let mut game_clock_start = GAME_CLOCK_START.lock().await;

        if *game_clock_start && !*OCR_API.lock().await {
            let time_diff = -1 * (Instant::now() - call_time).as_millis() as isize;
            if *game_clock as isize + time_diff >= 0 {
                *game_clock = (*game_clock as isize + time_diff) as usize;
            } else {
                *game_clock_start = false;
            }
        }
    }
}

pub async fn auto_update_checker() {
    loop {
        if let Ok(r) = update_checker().await {
            *OUT_OF_DATE.lock().await = r.0;
            *REMOTE_VERSION.lock().await = r.1;
        }

        tokio::time::sleep(std::time::Duration::from_secs(60 * 10)).await;
    }
}
