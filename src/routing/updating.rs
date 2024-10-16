use axum::{http::StatusCode, response::{Html, IntoResponse, Response}};
use anyhow::{Result, anyhow};

use crate::{printlg, appstate::global::*};

const REMOTE_CARGO_TOML_URL: &'static str =
    "https://raw.githubusercontent.com/AllLiver/Froggi/refs/heads/main/Cargo.toml";

pub async fn update_handler() -> impl IntoResponse {
    printlg!("Checking if an update is availible...");

    if let Ok(result) = update_checker().await {
        if result.0 {
            if let Some(tx) = UPDATE_SIGNAL.lock().await.take() {
                printlg!("Starting update...");
                let _ = tx.send(());
                return Response::builder()
                    .status(StatusCode::OK)
                    .body(String::from("Starting update..."))
                    .unwrap();
            } else {
                printlg!("Update signal already sent");
                return Response::builder()
                    .status(StatusCode::CONFLICT)
                    .body(String::from("Update already sent!"))
                    .unwrap();
            }
        } else {
            printlg!("Already up to date!");
            return Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(String::from("Already up to date!"))
                .unwrap();
        }
    } else {
        printlg!("Failed to make request to {}", REMOTE_CARGO_TOML_URL);
        return Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(format!(
                "Failed to make request to {}",
                REMOTE_CARGO_TOML_URL
            ))
            .unwrap();
    }
}

pub async fn update_checker() -> Result<(bool, String)> {
    let result = reqwest::get(REMOTE_CARGO_TOML_URL).await;

    if let Ok(response) = result {
        let remote_version_str_raw = response.text().await.expect("Failed to get response text");
        let remote_version_str = remote_version_str_raw
            .split("\n")
            .collect::<Vec<&str>>()
            .iter()
            .find(|x| x.starts_with("version = "))
            .expect("Failed to get remote version")
            .trim_start_matches("version = \"")
            .trim_end_matches("\"");

        let local_version_str = env!("CARGO_PKG_VERSION");

        let remote_version: Vec<u8> = remote_version_str
            .split(".")
            .map(|x| x.parse::<u8>().expect("Failed to parse remote version"))
            .collect();
        let local_version: Vec<u8> = local_version_str
            .split(".")
            .map(|x| x.parse::<u8>().expect("Failed to parse remote version"))
            .collect();

        let mut out_of_date = false;

        for i in 0..local_version.len() {
            if remote_version[i] > local_version[i] {
                out_of_date = true;
                break;
            } else if remote_version[i] < local_version[i] {
                break;
            }
        }

        return Ok((out_of_date, String::from(remote_version_str)));
    } else if let Err(e) = result {
        return Err(anyhow!("{}", e));
    } else {
        return Err(anyhow!("Some unexpected error"));
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

pub async fn update_menu_handler() -> impl IntoResponse {
    if *OUT_OF_DATE.lock().await {
        return Html::from(format!("
        <div class=\"settings-box\">
            <div class=\"settings-title\">
                <h2>Update</h2>
                <div class=\"subtext-settings\">
                    <h6>Update availible! ({} -> {})</h6>
                    <h6>Do not restart the server during update!</h6>
                </div>
            </div>
            <div class=\"setting\">
                <button hx-post=\"/update\" hx-confirm=\"You are trying to update Froggi, are you sure? This completely stops Froggi until the update is complete.\">Update now</button>
            </div>
        </div>
        ", 
        env!("CARGO_PKG_VERSION"),
        *REMOTE_VERSION.lock().await
    ));
    } else {
        return Html::from(String::from("<div class=\"settings-box\"><div class=\"settings-title\"><h2>Update</h2><div class=\"subtext-settings\"><h6>Froggi is up to date!</h6></div></div></div>"));
    }
}