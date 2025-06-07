use reqwest;
use image;
use serde::Deserialize;
use serde_json::from_reader;
use std::{env, fs::File};

use image::{DynamicImage, GenericImageView, GenericImage, imageops, imageops::FilterType};

use regex::Regex;
use reqwest::header::USER_AGENT;

use log::{info, warn};
use std::time::Instant;
use chrono::{Local, Datelike, Timelike};


#[derive(Deserialize, Debug)]
struct Aemet {
    key: String,
}

#[derive(Deserialize)]
struct AemetResponse {
    datos: String,
}

pub async fn get_image(url:&str) -> Result<image::DynamicImage, String> {
    let client = reqwest::Client::new();

    let img_bytes = match client
        .get(url)
        .header(USER_AGENT, "Mozilla/5.0 (Android 4.4; Mobile; rv:41.0) Gecko/41.0 Firefox/41.0")
        .send()
        .await {
            Ok(response) => {
                match response.bytes().await {
                    Ok(img_bytes) => img_bytes,
                    Err(err) => return Err(format!("Failed to read response bytes: {}", err)),
                }
            },
            Err(err) => return Err(format!("Failed to fetch image: {}", err)),
    };

    let image = match image::load_from_memory(&img_bytes) {
        Ok(img) => img,
        Err(err) => return Err(format!("Failed to load image: {}", err)),
    };

    Ok(image)
}

pub async fn get_image_url(key: String) -> Result<String, Box<dyn std::error::Error>> {
    let now = Local::now();

    let year = now.year();
    let month = now.month();
    let day = now.day();
    let hour = now.hour();
    let rounded_hour = (hour / 3) * 3;
    let url = format!("https://www.aemet.es//imagenes_d/eltiempo/prediccion/mod_maritima/{year:04}{month:02}{day:02}00+0{rounded_hour:02}_aewam_can_martot.png");
    Ok(url)
    // let url = format!("https://opendata.aemet.es/opendata/api/prediccion/maritima/costera/costa/43?api_key={key}");
    //
    // let client = reqwest::Client::new();
//
    // let response = match client
        // .get(url)
        // .header(USER_AGENT, "Mozilla/5.0 (Android 4.4; Mobile; rv:41.0) Gecko/41.0 Firefox/41.0")
        // .send()
        // .await {
            // Ok(response) => {
                // match response.text().await {
                    // Ok(img_bytes) => img_bytes,
                    // Err(err) => return Err(format!("Failed to read response bytes: {}", err).into()),
                // }
            // },
            // Err(err) => return Err(format!("Failed to fetch image: {}", err).into()),
    // };
//
    // let aresponse: AemetResponse = serde_json::from_str(&response)?;
    // println!("{response:?}");
//
    // return Ok(aresponse.datos);
}

pub async fn fetch_radar() -> Result<DynamicImage, String> {
    info!("Fetching radar...");

    let file = File::open("sensitive/aemet.json").expect("Unable to open aemet.json");
    let json: Aemet = from_reader(file).expect("Unable to parse aemet.json");
    let key = json.key.clone();

    if let Ok(image_url) = get_image_url(key).await {
        if let Ok(mut image1) = get_image(&image_url).await {
            // let image1 = hide_banner(&image1);
            // if env::var("NOT_KINDLE").is_err() {
                // image1 = image1.rotate90();
            // }


            // image1 = crop_right_square(&image1);
            image1 = crop_and_zoom_around_point(&image1, 670, 180, image1.height(), 4.0);

            return Ok(image1)
        } else {
            warn!("Could not load aemet image {image_url}");
            return Err(format!("Could not load aemet image"));
        }
    } else {
        warn!("Could not get image URL");
        return Err(format!("Could not get image URL"));
    }
}

pub async fn fetch_tides() -> Result<DynamicImage, String> {
    let image_url = format!("https://www.tideschart.com/tide-charts/en/Arrecife-Port-Provincia-de-Las-Palmas-Canary-Islands-Spain-tide-chart-14781240-m.png?date=20250607");
    if let Ok(mut image1) = get_image(&image_url).await {
        // let image1 = hide_banner(&image1);
        // if env::var("NOT_KINDLE").is_err() {
            // image1 = image1.rotate90();
        // }


        // image1 = crop_right_square(&image1);
        // image1 = crop_and_zoom_around_point(&image1, 670, 180, image1.height(), 4.0);

        return Ok(image1)
    } else {
        warn!("Could not load aemet image {image_url}");
        return Err(format!("Could not load aemet image"));
    }
}

fn crop_and_zoom_around_point(
    img: &DynamicImage,
    cx: u32,
    cy: u32,
    side: u32,
    zoom_factor: f32,
) -> DynamicImage {
    let (width, height) = img.dimensions();

    // Calculate the smaller crop size based on zoom factor
    let crop_side = (side as f32 / zoom_factor).round() as u32;

    // Clamp function
    let clamp = |val, min, max| {
        if val < min {
            min
        } else if val > max {
            max
        } else {
            val
        }
    };

    // Calculate top-left corner of crop rect
    let half_crop = crop_side / 2;
    let x = clamp(cx.saturating_sub(half_crop), 0, width.saturating_sub(crop_side));
    let y = clamp(cy.saturating_sub(half_crop), 0, height.saturating_sub(crop_side));

    // Crop the smaller region
    let cropped = img.crop_imm(x, y, crop_side, crop_side);

    // Resize cropped region back to original side length to simulate zoom
    cropped.resize_exact(side, side, FilterType::Lanczos3)
}

fn crop_right_square(img: &DynamicImage) -> DynamicImage {
    let (width, height) = img.dimensions();

    // The square side length is the height of the image
    let side = height;

    // Starting x coordinate to crop the right square
    // Make sure width >= height (landscape)
    let start_x = width.saturating_sub(side + 0);

    // Crop the image: x, y, width, height
    let cropped = img.crop_imm(start_x, 0, side, side);

    cropped
}
