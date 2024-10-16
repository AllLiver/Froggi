// Froggi routing (sponsors)

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

use crate::{appstate::global::*, id_create, printlg, utility::load_sponsors};

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
