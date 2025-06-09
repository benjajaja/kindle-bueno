use image::{self, ImageBuffer, Luma};
use reqwest;
use serde::Deserialize;

use image::{imageops::FilterType, DynamicImage, GenericImageView};

use reqwest::header::USER_AGENT;

use chrono::{Datelike, Local, Timelike};
use log::{info, warn};

#[derive(Deserialize, Debug)]
struct AemetKey {
    key: String,
}

#[derive(Deserialize, Debug)]
struct AemetRes {
    #[serde(rename = "estado")]
    status: u32,
    #[serde(rename = "datos")]
    data: String,
}

#[derive(Deserialize, Debug)]
struct AemetStation {
    #[serde(rename = "vv")]
    wind_speed: f32,
    #[serde(rename = "dv")]
    wind_direction: f32,
}

pub async fn get_image(url: &str) -> Result<image::DynamicImage, String> {
    let client = reqwest::Client::new();

    let img_bytes = match client
        .get(url)
        .header(
            USER_AGENT,
            "Mozilla/5.0 (Android 4.4; Mobile; rv:41.0) Gecko/41.0 Firefox/41.0",
        )
        .send()
        .await
    {
        Ok(response) => match response.bytes().await {
            Ok(img_bytes) => img_bytes,
            Err(err) => return Err(format!("Failed to read response bytes: {}", err)),
        },
        Err(err) => return Err(format!("Failed to fetch image: {}", err)),
    };

    let image = match image::load_from_memory(&img_bytes) {
        Ok(img) => img,
        Err(err) => return Err(format!("Failed to load image: {}", err)),
    };

    Ok(image)
}

pub async fn get_image_url() -> Result<String, Box<dyn std::error::Error>> {
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

    // let file = File::open("sensitive/aemet.json").expect("Unable to open aemet.json");
    // let json: Aemet = from_reader(file).expect("Unable to parse aemet.json");
    // let key = json.key.clone();

    if let Ok(image_url) = get_image_url().await {
        if let Ok(mut image1) = get_image(&image_url).await {
            // let image1 = hide_banner(&image1);
            // if env::var("NOT_KINDLE").is_err() {
            // image1 = image1.rotate90();
            // }

            // image1 = crop_right_square(&image1);
            // image1 = crop_and_zoom_around_point(&image1, 670, 180, image1.height(), 2.0);
            image1 = DynamicImage::ImageLuma8(remap_colors_to_grayscale_fuzzy(&image1));

            return Ok(image1);
        } else {
            warn!("Could not load aemet image {image_url}");
            return Err(format!("Could not load aemet image"));
        }
    } else {
        warn!("Could not get image URL");
        return Err(format!("Could not get image URL"));
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
    let x = clamp(
        cx.saturating_sub(half_crop),
        0,
        width.saturating_sub(crop_side),
    );
    let y = clamp(
        cy.saturating_sub(half_crop),
        0,
        height.saturating_sub(crop_side),
    );

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

/// Computes squared Euclidean distance between two RGB colors
fn color_distance_sq(c1: (u8, u8, u8), c2: (u8, u8, u8)) -> u32 {
    let dr = c1.0 as i32 - c2.0 as i32;
    let dg = c1.1 as i32 - c2.1 as i32;
    let db = c1.2 as i32 - c2.2 as i32;
    (dr * dr + dg * dg + db * db) as u32
}

fn remap_colors_to_grayscale_fuzzy(img: &DynamicImage) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let color_map = [
        ((205, 255, 255), 255), // light teal - brightest
        ((0, 0, 254), 242),     // new blue, between light teal and teal
        ((129, 243, 255), 230), // teal
        ((0, 255, 0), 190),     // green
        ((255, 255, 75), 160),  // yellow
        ((255, 218, 0), 140),   // light orange
        ((255, 181, 0), 120),   // orange
        ((255, 0, 0), 80),      // red
        ((231, 0, 129), 60),    // purple
        ((181, 0, 181), 40),    // dark purple
    ];

    let (width, height) = img.dimensions();
    let mut gray_img = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.pixels() {
        let rgb = (pixel[0], pixel[1], pixel[2]);

        // Find nearest color in color_map by minimal distance
        let mut best_gray = 255;
        let mut best_dist = u32::MAX;
        for (col, gray_val) in &color_map {
            let dist = color_distance_sq(rgb, *col);
            if dist < best_dist {
                best_dist = dist;
                best_gray = *gray_val;
            }
        }

        gray_img.put_pixel(x, y, Luma([best_gray]));
    }

    gray_img
}

#[derive(Debug)]
pub struct Wind {
    pub speed: f32,
    pub direction: f32,
}

pub async fn fetch_wind() -> Result<Wind, Box<dyn std::error::Error>> {
    let file = std::fs::File::open("sensitive/aemet.json")?;
    let json_key: AemetKey = serde_json::from_reader(file)?;
    let key = json_key.key;

    let url = format!(
        "https://opendata.aemet.es/opendata/api/observacion/convencional/datos/estacion/C029O?api_key={key}"
    );

    let client = reqwest::Client::new();

    let response = client.get(url).send().await?;
    let ares: AemetRes = response.json().await?;

    if ares.status != 200 {
        return Err(format!("aemet status {}", ares.status).into());
    }

    let url = ares.data;

    let response = client.get(url).send().await?;
    let data: Vec<AemetStation> = response.json().await?;

    let Some(last) = data.last() else {
        return Err(format!("no aemet data").into());
    };
    Ok(Wind {
        speed: last.wind_speed,
        direction: last.wind_direction,
    })
}
