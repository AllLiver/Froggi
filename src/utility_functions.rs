use std::time::UNIX_EPOCH;

use rand::{thread_rng, Rng};

use crate::{Config, appstate::global::*};

pub fn id_create(l: u8) -> String {
    const BASE62: &'static str = "qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890";

    let mut id = String::new();
    let base62: Vec<char> = BASE62.chars().collect();

    for _ in 0..l {
        id.push(base62[thread_rng().gen_range(0..base62.len())])
    }

    id
}

pub async fn program_lock() -> Result<(), std::io::Error> {
    let time = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    tokio::fs::write("./tmp/froggi.lock", time.to_string()).await
}

pub async fn release_program_lock() -> Result<(), std::io::Error> {
    tokio::fs::remove_file("./tmp/froggi.lock").await
}

pub async fn update_program_lock() {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let time = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        tokio::fs::write("./tmp/froggi.lock", time.to_string())
            .await
            .expect("Failed to update froggi.lock");
    }
}

pub async fn load_config() {
    let config: Config = serde_json::from_str(
        &tokio::fs::read_to_string("./config.json")
            .await
            .expect("Failed to read config.json"),
    )
    .expect("Failed to deserialize config.json");

    *COUNTDOWN_OPACITY.lock().await = config.countdown_opacity;
}

pub fn hex_to_rgb(hex: &String) -> (u8, u8, u8) {
    let hex_chars: Vec<char> = hex.trim_start_matches("#").to_string().chars().collect();

    let r = hex_char_to_u8(hex_chars[0]) * 16 + hex_char_to_u8(hex_chars[1]);
    let g = hex_char_to_u8(hex_chars[2]) * 16 + hex_char_to_u8(hex_chars[3]);
    let b = hex_char_to_u8(hex_chars[4]) * 16 + hex_char_to_u8(hex_chars[5]);

    (r, g, b)
}

pub fn hex_char_to_u8(c: char) -> u8 {
    match c {
        '0' => 0,
        '1' => 1,
        '2' => 2,
        '3' => 3,
        '4' => 4,
        '5' => 5,
        '6' => 6,
        '7' => 7,
        '8' => 8,
        '9' => 9,
        'A' => 10,
        'B' => 11,
        'C' => 12,
        'D' => 13,
        'E' => 14,
        'F' => 15,
        'a' => 10,
        'b' => 11,
        'c' => 12,
        'd' => 13,
        'e' => 14,
        'f' => 15,
        _ => 15,
    }
}

pub fn rgb_to_hex(rgb: &(u8, u8, u8)) -> String {
    format!(
        "#{}{}{}{}{}{}",
        u8_to_hex_char((rgb.0 - (rgb.0 % 16)) / 16),
        u8_to_hex_char(rgb.0 % 16),
        u8_to_hex_char((rgb.1 - (rgb.1 % 16)) / 16),
        u8_to_hex_char(rgb.1 % 16),
        u8_to_hex_char((rgb.2 - (rgb.2 % 16)) / 16),
        u8_to_hex_char(rgb.2 % 16)
    )
}

pub fn u8_to_hex_char(u: u8) -> char {
    match u {
        0 => '0',
        1 => '1',
        2 => '2',
        3 => '3',
        4 => '4',
        5 => '5',
        6 => '6',
        7 => '7',
        8 => '8',
        9 => '9',
        10 => 'A',
        11 => 'B',
        12 => 'C',
        13 => 'D',
        14 => 'E',
        15 => 'F',
        _ => 'F',
    }
}