// Froggi tickers

use tokio::{sync::MutexGuard, time::Instant};

use crate::{appstate::global::*, update_checker};

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
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(
            *SPONSOR_WAIT_TIME.lock().await,
        ))
        .await;
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

pub async fn countdown_clock_process() {
    let now = Instant::now();
    *COUNTDOWN_CLOCK_END_INSTANT.lock().await =
        now + std::time::Duration::from_millis(*COUNTDOWN_CLOCK.lock().await as u64);

    loop {
        let mut countdown_clock_start = COUNTDOWN_CLOCK_START.lock().await;
        if *countdown_clock_start {
            let new_time = COUNTDOWN_CLOCK_END_INSTANT
                .lock()
                .await
                .duration_since(Instant::now())
                .as_millis() as u64;

            if new_time == 0 {
                *countdown_clock_start = false;
            }

            *COUNTDOWN_CLOCK.lock().await = new_time;

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

pub async fn set_countdown_clock(mutex: &mut MutexGuard<'_, u64>, millis: u64) {
    if *COUNTDOWN_CLOCK_START.lock().await {
        *COUNTDOWN_CLOCK_END_INSTANT.lock().await =
            Instant::now() + std::time::Duration::from_millis(millis);
    } else {
        **mutex = millis;
    }
}

pub async fn start_countdown_clock() {
    let now = Instant::now();

    *COUNTDOWN_CLOCK_END_INSTANT.lock().await =
        now + std::time::Duration::from_millis(*COUNTDOWN_CLOCK.lock().await as u64);

    *COUNTDOWN_CLOCK_START.lock().await = true;
}

pub async fn stop_countdown_clock() {
    *COUNTDOWN_CLOCK_START.lock().await = false;
}

pub async fn game_clock_process() {
    let now = Instant::now();
    *GAME_CLOCK_END_INSTANT.lock().await =
        now + std::time::Duration::from_millis(*GAME_CLOCK.lock().await as u64);

    loop {
        let mut game_clock_start = GAME_CLOCK_START.lock().await;
        if *game_clock_start {
            let new_time = GAME_CLOCK_END_INSTANT
                .lock()
                .await
                .duration_since(Instant::now())
                .as_millis() as u64;

            if new_time == 0 {
                *game_clock_start = false;
            }

            *GAME_CLOCK.lock().await = new_time;

            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }
}

pub async fn set_game_clock(mutex: &mut MutexGuard<'_, u64>, millis: u64) {
    if *GAME_CLOCK_START.lock().await {
        *GAME_CLOCK_END_INSTANT.lock().await =
            Instant::now() + std::time::Duration::from_millis(millis);
    } else {
        **mutex = millis;
    }
}

pub async fn start_game_clock() {
    let now = Instant::now();

    *GAME_CLOCK_END_INSTANT.lock().await =
        now + std::time::Duration::from_millis(*GAME_CLOCK.lock().await as u64);

    *GAME_CLOCK_START.lock().await = true;
}

pub async fn stop_game_clock() {
    *GAME_CLOCK_START.lock().await = false;
    let mut game_clock = GAME_CLOCK.lock().await;
    if *game_clock >= 1000 * 60 {
        *game_clock = *game_clock / 1000 * 1000;
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
