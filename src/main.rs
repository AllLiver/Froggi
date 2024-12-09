// Froggi main

#[forbid(unsafe_code)]
use anyhow::Context;
use anyhow::Result;
use base64::prelude::*;
use rand::Rng;
use tokio::{
    fs::{create_dir_all, File},
    io::AsyncWriteExt,
    signal,
    sync::oneshot,
};

mod appstate;
mod routing;
mod tickers;
mod utility;

use crate::appstate::global::*;
use crate::appstate::routing::*;

use crate::utility::hex::*;
use crate::utility::login::*;
use crate::utility::*;

use crate::routing::froggi_router;

use crate::tickers::*;

#[macro_export]
macro_rules! printlg {
    ($($arg:tt)*) => {{
        use std::fmt::Write;

        let mut buffer = String::new();
        write!(&mut buffer, $($arg)*).expect("Failed to write to buffer");

        LOGS.lock().await.push(buffer.clone());

        println!("{}", buffer);
    }};
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    // Verify tmp directory exists
    create_dir_all("./tmp").await?;

    let (restart_tx, restart_rx) = oneshot::channel();
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let (update_tx, update_rx) = oneshot::channel();

    // Initialize the application state
    let mut state = AppState::default();

    if let Ok(s) = AppState::load_saved_state().await {
        state = s;
    }

    *RESTART_SIGNAL.lock().await = Some(restart_tx);
    *SHUTDOWN_SIGNAL.lock().await = Some(shutdown_tx);
    *UPDATE_SIGNAL.lock().await = Some(update_tx);

    // Validate required files and directories
    if let Err(_) = File::open("secret.key").await {
        printlg!("Initializing secret.key");
        let mut f = File::create("secret.key").await?;

        let key: [u8; 32] = rand::thread_rng().gen();
        let secret = BASE64_STANDARD.encode(key);

        f.write_all(secret.as_bytes()).await?;
    }

    match File::open("config.json").await {
        Ok(_) => {
            let cfg: Config =
                serde_json::from_str(&tokio::fs::read_to_string("config.json").await?)?;

            if cfg.secure_auth_cookie == false {
                printlg!("! ! ! ! ! ! ! ! ! !");
                printlg!("WARNING! DISABLING SECURE AUTH COOKIE IN config.json COULD RESULT IN SENDING LOGIN CREDENTIALS OVER UNENCRYPTED TRAFFIC, THIS IS UNSAFE AND SHOULD ONLY BE USED FOR DEVELOPMENT PURPOSES! UNLESS YOU KNOW WHAT YOU ARE DOING, PLEASE ENABLE SECURE AUTH COOKIE.");
                printlg!("! ! ! ! ! ! ! ! ! !");
            }
        }
        Err(_) => {
            printlg!("Initializing config.json");
            let mut f = File::create("config.json").await?;

            let default_config = Config {
                secure_auth_cookie: true,
                sponsor_wait_time: 5,
                countdown_opacity: 0.5,
                popup_opacity: 0.5,
            };

            f.write_all(serde_json::to_string_pretty(&default_config)?.as_bytes())
                .await?;
        }
    }

    create_dir_all(format!("./sponsors")).await?;

    create_dir_all(format!("./team-presets")).await?;

    // Load sponsor img tags
    load_sponsors().await;
    load_config().await;

    let app = froggi_router(&state);

    if let Ok(listener) = tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        tokio::spawn(game_clock_ticker());
        tokio::spawn(countdown_clock_ticker());
        tokio::spawn(sponsor_ticker());
        tokio::spawn(popup_home_ticker());
        tokio::spawn(popup_away_ticker());
        tokio::spawn(auto_update_checker());
        printlg!(" -> LISTENING ON: 0.0.0.0:3000");

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal(restart_rx, shutdown_rx, update_rx))
            .await
            .context("Could not serve app")?;
    } else {
        panic!("Could not bind tcp listener!");
    }

    printlg!("Saving app state...");

    if let Ok(save_app_state) =
        serde_json::to_string(&AppStateSerde::consume_app_state(state).await)
    {
        if let Ok(_) = tokio::fs::write("./appstate.json", save_app_state).await {
            printlg!("Saved app state!");
        } else {
            printlg!("Failed to save app state!");
        }
    } else {
        printlg!("Failed to save app state!");
    }

    if let Some(code) = EXIT_CODE.lock().await.take() {
        if code == 10 {
            printlg!("Shut down gracefully\n");
        } else {
            printlg!("Shut down gracefully");
        }
        std::process::exit(code);
    }

    printlg!("Shut down gracefully");

    Ok(())
}

// Code borrowed from https://github.com/tokio-rs/axum/blob/806bc26e62afc2e0c83240a9e85c14c96bc2ceb3/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal(
    mut restart_rx: oneshot::Receiver<()>,
    mut shutdown_rx: oneshot::Receiver<()>,
    mut update_rx: oneshot::Receiver<()>,
) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = async {
        let _ = std::future::pending::<()>().await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = &mut restart_rx => {
            *EXIT_CODE.lock().await = Some(10);
        }
        _ = &mut shutdown_rx => {
            *EXIT_CODE.lock().await = Some(0);
        }
        _ = &mut update_rx => {
            *EXIT_CODE.lock().await = Some(11);
        }
    }
}
