// Froggi routing (teaminfo)

use axum::{
    body::Body, extract::{Multipart, Path, State}, http::{HeaderName, HeaderValue, StatusCode}, response::{Html, IntoResponse, Response}
};
use base64::prelude::*;
use flate2::{Compression, GzBuilder};
use reqwest::header::CONTENT_DISPOSITION;
use std::{io::Write, path::PathBuf};
use tokio_util::io::ReaderStream;
use tokio::{
    fs::{create_dir_all, read_dir, remove_dir_all, File},
    io::{AsyncReadExt, AsyncWriteExt, BufReader}, task::spawn_blocking,
};

use crate::appstate::global::*;
use crate::{hex_to_rgb, id_create, printlg, rgb_to_hex, utility::Teaminfo, AppState};

pub async fn teaminfo_preset_create_handler(mut form: Multipart) -> impl IntoResponse {
    let mut teaminfo = Teaminfo::new();
    let id = id_create(12);

    create_dir_all(format!("team-presets/{}", id))
        .await
        .expect("Could not create team preset directory");

    while let Some(field) = form
        .next_field()
        .await
        .expect("Could not get next field of preset create multipart")
    {
        match field.name().unwrap() {
            "home_name" => {
                teaminfo.home_name = field.text().await.unwrap();
            }
            "home_img" => {
                let mut f = File::create(format!(
                    "team-presets/{}/home.{}",
                    id,
                    field
                        .file_name()
                        .unwrap()
                        .to_string()
                        .split(".")
                        .collect::<Vec<&str>>()[1]
                ))
                .await
                .expect("Could not create home img");

                f.write_all(field.bytes().await.unwrap().as_ref())
                    .await
                    .expect("Could not write to home img");
            }
            "home_color" => {
                teaminfo.home_color = field.text().await.unwrap();
            }
            "away_name" => {
                teaminfo.away_name = field.text().await.unwrap();
            }
            "away_img" => {
                let mut f = File::create(format!(
                    "team-presets/{}/away.{}",
                    id,
                    field
                        .file_name()
                        .unwrap()
                        .to_string()
                        .split(".")
                        .collect::<Vec<&str>>()[1]
                ))
                .await
                .expect("Could not create away img");

                f.write_all(field.bytes().await.unwrap().as_ref())
                    .await
                    .expect("Could not write to away img");
            }
            "away_color" => {
                teaminfo.away_color = field.text().await.unwrap();
            }
            _ => {}
        }
    }

    let write_json = serde_json::to_string_pretty(&teaminfo).expect("Could not serialize teaminfo");

    let mut f = File::create(format!("team-presets/{}/teams.json", id))
        .await
        .expect("Could not create teams.json");
    f.write_all(write_json.as_bytes())
        .await
        .expect("Could not write to teams.json");

    printlg!(
        "CREATE teaminfo_preset: {} vs {} (id: {})",
        teaminfo.home_name,
        teaminfo.away_name,
        id
    );

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-selector"),
        )
        .body(String::new())
        .unwrap();
}

pub async fn teaminfo_preset_selector_handler() -> impl IntoResponse {
    let mut html = String::new();
    let mut a = read_dir("./team-presets").await.unwrap();

    while let Ok(Some(d)) = a.next_entry().await {
        if d.file_type()
            .await
            .expect("Could not get preset file type")
            .is_dir()
        {
            let mut home_img_path = PathBuf::new();
            let mut away_img_path = PathBuf::new();

            let mut home_tag_type = "";
            let mut away_tag_type = "";

            let mut teaminfo = Teaminfo::new();

            let mut b = read_dir(d.path()).await.unwrap();

            while let Ok(Some(d0)) = b.next_entry().await {
                let file_name = d0.file_name().to_string_lossy().to_string();

                if file_name.starts_with("home.") {
                    home_img_path = d0.path();

                    home_tag_type = match file_name.split(".").collect::<Vec<&str>>()[1] {
                        "png" => "png",
                        "jpg" => "jpeg",
                        "jpeg" => "jpeg",
                        _ => "",
                    }
                } else if file_name.starts_with("away.") {
                    away_img_path = d0.path();

                    away_tag_type = match file_name.split(".").collect::<Vec<&str>>()[1] {
                        "png" => "png",
                        "jpg" => "jpeg",
                        "jpeg" => "jpeg",
                        _ => "",
                    }
                } else if file_name == "teams.json" {
                    let f = File::open(d0.path())
                        .await
                        .expect("Could not open teams.json");
                    let mut buf_reader = BufReader::new(f);

                    let mut temp_str = String::new();

                    buf_reader
                        .read_to_string(&mut temp_str)
                        .await
                        .expect("Could not read teams.json");

                    teaminfo =
                        serde_json::from_str(&temp_str).expect("Could not deserialize teams.json");
                }
            }
            let home_img_bytes = tokio::fs::read(home_img_path)
                .await
                .expect("Could not read home img");
            let away_img_bytes = tokio::fs::read(away_img_path)
                .await
                .expect("Could not read away img");

            let id = d.file_name().to_string_lossy().to_string();

            html += &format!(
            "<div class=\"ti-match-selector\">
                <img class=\"home-logo\" src=\"data:image/{};base64,{}\" alt=\"home-img\" height=\"30px\" width=\"auto\" style=\"border-color: {}; border-style: solid; border-radius: 3px; border-width: 2px\">
                <p class=\"teampreset-title\">{} vs {}</p>
                <img class=\"away-logo\" src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"30px\" width=\"auto\" style=\"border-color: {}; border-style: solid; border-radius: 3px; border-width: 2px;\">
                <button class=\"select-button\" hx-post=\"/teaminfo/select/{}\" hx-swap=\"none\">Select</button>
                <button class=\"remove-button\" hx-post=\"/teaminfo/remove/{}\" hx-swap=\"none\">Remove</button>
                <a href=\"/teaminfo/download-preset/{}\">Download</button>
            </div>",
                home_tag_type,
                BASE64_STANDARD.encode(home_img_bytes),
                teaminfo.home_color,
                teaminfo.home_name,
                teaminfo.away_name,
                away_tag_type,
                BASE64_STANDARD.encode(away_img_bytes),
                teaminfo.away_color,
                id,
                id,
                id
            );
        }
    }

    return Html::from(html);
}

pub async fn teaminfo_preset_select_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let mut dir = read_dir("./team-presets").await.unwrap();

    while let Ok(Some(a)) = dir.next_entry().await {
        if a.file_type()
            .await
            .expect("Could not get file type of dir entry")
            .is_dir()
        {
            if a.file_name().to_string_lossy().to_string() == id {
                *state.preset_id.lock().await = id.clone();

                let mut a_json = String::new();

                let a_json_f = File::open(format!(
                    "{}/teams.json",
                    a.path().to_string_lossy().to_string()
                ))
                .await
                .expect("Could not open preset file");
                let mut buf_reader = BufReader::new(a_json_f);

                buf_reader
                    .read_to_string(&mut a_json)
                    .await
                    .expect("Could not read preset file");

                let team_info: Teaminfo =
                    serde_json::from_str(&a_json).expect("Could not deserialize preset file");

                printlg!(
                    "SELECT teaminfo_preset: {} vs {} (id: {})",
                    team_info.home_name,
                    team_info.away_name,
                    id
                );

                *state.home_name.lock().await = team_info.home_name;
                *state.away_name.lock().await = team_info.away_name;

                break;
            }
        }
    }

    return StatusCode::OK;
}

pub async fn teaminfo_preset_remove_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Ok(_) = remove_dir_all(format!("./team-presets/{}", id)).await {
        *state.preset_id.lock().await = String::new();
        printlg!("REMOVE teaminfo_preset: {}", id);
    }

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-selector"),
        )
        .body(String::new())
        .unwrap();
}

pub async fn team_name_display_handler(
    Path(t): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    if t == "home" {
        return Html::from(state.home_name.lock().await.clone());
    } else if t == "away" {
        return Html::from(state.away_name.lock().await.clone());
    } else {
        return Html::from(String::new());
    }
}

pub async fn teaminfo_button_css_handler(State(state): State<AppState>) -> impl IntoResponse {
    let preset_id = state.preset_id.lock().await;
    if preset_id.is_empty() {
        return Html::from(String::new());
    } else {
        if let Ok(teaminfo) = serde_json::from_str::<Teaminfo>(
            &tokio::fs::read_to_string(format!("./team-presets/{}/teams.json", *preset_id))
                .await
                .unwrap(),
        ) {
            let home_rgb = hex_to_rgb(&teaminfo.home_color);
            let home_text_color = rgb_to_hex(&(255 - home_rgb.0, 255 - home_rgb.1,  255 - home_rgb.2));

            let away_rgb = hex_to_rgb(&teaminfo.away_color);
            let away_text_color = rgb_to_hex(&(255 - away_rgb.0, 255 - away_rgb.1,  255 - away_rgb.2));

            return Html::from(format!(
                "
            <style>
                .button-decrement-home {{
                    background-color: {};
                    color: {};
                }}
                .button-increment-home {{
                    background-color: {};
                    color: {};
                }}
                .button-preset-score-home {{
                    background-color: {};
                    color: {};
                }}
                .button-decrement-away {{
                    background-color: {};
                    color: {};
                }}
                .button-increment-away {{
                    background-color: {};
                    color: {};
                }}
                .button-preset-score-away {{
                    background-color: {};
                    color: {};
                }}
                .trigger-home-popup {{
                    background-color: {};
                    color: {};
                }}
                .trigger-away-popup {{
                    background-color: {};
                    color: {};
                }}
            </style>
            ",
                teaminfo.home_color,
                home_text_color,
                teaminfo.home_color,
                home_text_color,
                teaminfo.home_color,
                home_text_color,
                teaminfo.away_color,
                away_text_color,
                teaminfo.away_color,
                away_text_color,
                teaminfo.away_color,
                away_text_color,
                teaminfo.home_color,
                home_text_color,
                teaminfo.away_color,
                away_text_color,
            ));
        } else {
            return Html::from(String::new());
        }
    }
}

pub async fn teaminfo_download_preset_handler(Path(a): Path<String>) -> impl IntoResponse {
    let ti: Teaminfo = serde_json::from_str(&tokio::fs::read_to_string(&format!("./team-presets/{}/teams.json", a)).await.expect("Failed to read team preset file")).expect("Failed to serialize team preset file");
    
    let teaminfo: Teaminfo = ti.clone();
    let id = a.clone();
    
    spawn_blocking(move || {    
        let tar_archive_path = format!("./tmp/{}.tar", id);
        
        let tar_archive = std::fs::File::create(&tar_archive_path).expect("Failed to create team preset tar file");
        let mut builder = tar::Builder::new(tar_archive);
        
        let mut dir = std::fs::read_dir(format!("./team-presets/{}", id)).expect("Failed to read team preset directory");
        
        while let Some(Ok(f)) = dir.next() {
            let mut file: std::fs::File = std::fs::File::open(f.path()).expect("Failed to open team preset file");
            
            builder.append_file(format!("./{}", f.file_name().to_string_lossy().to_string()), &mut file).expect("Failed to append team preset file to team preset archive");
        }
        
        builder.finish().expect("Failed to write team preset archive");
        
        let gz_path = format!("{}.gz", tar_archive_path);
        
        let gz_file = std::fs::File::create(&gz_path).expect("Failed to create team preset gzip");
        let mut gz = GzBuilder::new()
            .filename(format!("{}-{}.tar", teaminfo.home_name.clone(), teaminfo.away_name.clone()))
            .write(gz_file, Compression::default());
        
        gz.write_all(&std::fs::read(tar_archive_path).expect("Failed to read preset tar file (for compression)")).expect("Failed to write tar archive to team preset gzip");
        gz.finish().expect("Failed to finish gz archive");
        
    }).await.expect("Failed to create team preset archive");
    
    let teaminfo_archive = ReaderStream::new(tokio::fs::File::open(format!("./tmp/{}.tar.gz", a)).await.expect("Failed to open gz preset archive"));
    
    tokio::fs::remove_file(format!("./tmp/{}.tar.gz", a)).await.expect("Failed to remove gz preset archive (remove from /tmp asap)");
    tokio::fs::remove_file(format!("./tmp/{}.tar", a)).await.expect("Failed to remove tar preset archive (remove from /tmp asap)");

    return Response::builder()
        .header(CONTENT_DISPOSITION, format!("attachment; filename=\"{}-{}.tar.gz\"", ti.home_name, ti.away_name))
        .header("content-type", "application/octet-stream")
        .body(Body::from_stream(teaminfo_archive))
        .unwrap();
}
