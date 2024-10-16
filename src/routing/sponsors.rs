use std::path::PathBuf;

use axum::{
    extract::{Multipart, Path},
    http::{HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};
use base64::prelude::*;
use tokio::{
    fs::{create_dir_all, read_dir, remove_file, File},
    io::AsyncWriteExt,
};

use crate::{appstate::global::*, id_create, printlg, Config};

pub async fn upload_sponsors_handler(mut form: Multipart) -> impl IntoResponse {
    create_dir_all(format!("./sponsors"))
        .await
        .expect("Could not create sponsors directory");

    while let Some(field) = form
        .next_field()
        .await
        .expect("Could not get next field of sponsor multipart")
    {
        let id = id_create(12);
        let mut f = File::create(format!(
            "./sponsors/{}.{}",
            id,
            field.file_name().unwrap().split(".").collect::<Vec<&str>>()[1]
        ))
        .await
        .expect("Could not create sponsor file");

        f.write_all(field.bytes().await.unwrap().as_ref())
            .await
            .expect("Could not write to sponsor file");

        println!("ADD sponsor: {}", id);
    }

    load_sponsors().await;

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-sponsor"),
        )
        .body(String::new())
        .unwrap();
}

pub async fn sponsors_management_handler() -> impl IntoResponse {
    let mut d = read_dir("./sponsors").await.unwrap();
    let mut html = String::new();

    while let Ok(Some(a)) = d.next_entry().await {
        let fname = a.file_name().to_string_lossy().to_string();
        let fname_vec = fname.split(".").collect::<Vec<&str>>();

        let mime = match fname_vec[1] {
            "png" => "png",
            "jpg" => "jpeg",
            "jpeg" => "jpeg",
            _ => "",
        };

        let f_bytes = tokio::fs::read(a.path())
            .await
            .expect("Could not read sponsor image");

        html += &format!(
            "<div class=\"sponsor-wrapper\">
                <img src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"30px\" width=\"auto\">
                <button class=\"remove-button\" hx-post=\"/sponsors/remove/{}\" hx-swap=\"none\">Remove</button>
            </div>",
            mime,
            BASE64_STANDARD.encode(f_bytes),
            fname_vec[0]
        );
    }

    return Html::from(html);
}

pub async fn sponsor_remove_handler(Path(id): Path<String>) -> impl IntoResponse {
    let mut d = read_dir("./sponsors").await.unwrap();
    let mut p = PathBuf::new();

    while let Ok(Some(a)) = d.next_entry().await {
        if a.file_name()
            .to_string_lossy()
            .to_string()
            .split(".")
            .collect::<Vec<&str>>()[0]
            == id
        {
            p = a.path();
            break;
        }
    }

    remove_file(p).await.expect("Could not remove sponsor file");

    printlg!("REMOVE sponsor: {}", id);

    return Response::builder()
        .status(StatusCode::OK)
        .header(
            HeaderName::from_static("hx-trigger"),
            HeaderValue::from_static("reload-sponsor"),
        )
        .body(String::new())
        .unwrap();
}

pub async fn load_sponsors() {
    create_dir_all(format!("./sponsors"))
        .await
        .expect("Could not create sponsors directory");

    let mut d = read_dir("./sponsors")
        .await
        .expect("Could not read sponsors dir");

    while let Ok(Some(f)) = d.next_entry().await {
        let fname = f.file_name().to_string_lossy().to_string();

        let mime_type = match fname.split(".").collect::<Vec<&str>>()[1] {
            "png" => "png",
            "jpg" => "jpeg",
            "jpeg" => "jpeg",
            _ => "",
        };

        let f_bytes = tokio::fs::read(f.path())
            .await
            .expect("Could not read sponsor image");

        *SPONSOR_IDX.lock().await = 0;
        SPONSOR_TAGS.lock().await.push(format!(
            "<img class=\"ol-sponsor-img\" src=\"data:image/{};base64,{}\" alt=\"away-img\" height=\"auto\">",
            mime_type,
            BASE64_STANDARD.encode(f_bytes),
        ))
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
